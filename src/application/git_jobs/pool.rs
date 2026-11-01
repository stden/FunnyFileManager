//! Purpose: Worker-pool queue and thread lifecycle for background `git status` probes.

use std::collections::{HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc, Condvar, Mutex};

use crate::models::FileName;

pub(super) type Task = (PathBuf, FileName);
pub(super) type Outcome = (PathBuf, FileName, bool);
pub(super) type Prober = Arc<dyn Fn(&Path, &FileName) -> bool + Send + Sync>;

#[derive(Default)]
struct Queue {
    tasks: VecDeque<Task>,
    busy: usize,
    closed: bool,
}

type SharedQueue = Arc<(Mutex<Queue>, Condvar)>;

#[cfg(any(test, tarpaulin))]
thread_local! {
    pub(super) static FORCE_SPAWN_FAILURE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Small batches are cheaper to compare directly than to index a large queue.
const LINEAR_BATCH_LIMIT: usize = 12;

pub(super) struct Pool {
    shared: SharedQueue,
    outcomes: mpsc::Receiver<Outcome>,
    sender: mpsc::Sender<Outcome>,
    workers: usize,
}

impl Pool {
    pub(super) fn new() -> Self {
        let (sender, outcomes) = mpsc::channel();
        Self {
            shared: Arc::new((Mutex::new(Queue::default()), Condvar::new())),
            outcomes,
            sender,
            workers: 0,
        }
    }

    pub(super) fn close(&self) {
        let (lock, wake) = &*self.shared;
        if let Ok(mut queue) = lock.lock() {
            queue.tasks.clear();
            queue.closed = true;
            wake.notify_all();
        }
    }

    pub(super) fn try_recv(&self) -> Result<Outcome, mpsc::TryRecvError> {
        self.outcomes.try_recv()
    }

    pub(super) fn update_queue(
        &self,
        active: [&Path; 2],
        batches: [Option<(PathBuf, Vec<FileName>)>; 2],
    ) -> usize {
        let (lock, wake) = &*self.shared;
        let Ok(mut queue) = lock.lock() else {
            return 0;
        };
        queue
            .tasks
            .retain(|(parent, _)| active.contains(&parent.as_path()));
        let added = push_unique(&mut queue.tasks, batches);
        let demand = queue.tasks.len().saturating_add(queue.busy);
        drop(queue);
        if added > 0 {
            wake.notify_all();
        }
        demand
    }

    pub(super) fn grow_to(&mut self, target: usize, prober: &Prober) {
        while self.workers < target {
            let spawned = spawn_worker(
                Arc::clone(&self.shared),
                self.sender.clone(),
                Arc::clone(prober),
            );
            if spawned.is_err() {
                break;
            }
            self.workers += 1;
        }
    }
}

fn spawn_worker(
    shared: SharedQueue,
    sender: mpsc::Sender<Outcome>,
    prober: Prober,
) -> std::io::Result<std::thread::JoinHandle<()>> {
    #[cfg(any(test, tarpaulin))]
    if FORCE_SPAWN_FAILURE.get() {
        return Err(std::io::Error::other("forced spawn failure"));
    }
    std::thread::Builder::new()
        .name("ffm-git".to_owned())
        .spawn(move || worker_loop(&shared, &sender, &prober))
}

fn push_unique(
    tasks: &mut VecDeque<Task>,
    batches: [Option<(PathBuf, Vec<FileName>)>; 2],
) -> usize {
    let incoming = batches
        .iter()
        .flatten()
        .map(|(_, names)| names.len())
        .sum::<usize>();
    if incoming <= LINEAR_BATCH_LIMIT {
        return batches
            .into_iter()
            .flatten()
            .map(|(parent, names)| push_small_batch(tasks, &parent, names))
            .sum();
    }
    let mut seen: HashSet<Task> = tasks.iter().cloned().collect();
    let mut added = 0;
    for (parent, names) in batches.into_iter().flatten() {
        for name in names {
            let task = (parent.clone(), name);
            if seen.insert(task.clone()) {
                tasks.push_back(task);
                added += 1;
            }
        }
    }
    added
}

fn push_small_batch(tasks: &mut VecDeque<Task>, parent: &Path, names: Vec<FileName>) -> usize {
    let mut added = 0;
    for name in names {
        let duplicate = tasks
            .iter()
            .any(|(queued_parent, queued_name)| queued_parent == parent && *queued_name == name);
        if !duplicate {
            tasks.push_back((parent.to_path_buf(), name));
            added += 1;
        }
    }
    added
}

fn worker_loop(shared: &SharedQueue, sender: &mpsc::Sender<Outcome>, prober: &Prober) {
    while let Some((parent, name)) = next_task(shared) {
        let dirty = prober(&parent, &name);
        finish_task(shared);
        if sender.send((parent, name, dirty)).is_err() {
            break;
        }
    }
}

fn next_task(shared: &SharedQueue) -> Option<Task> {
    let (lock, wake) = &**shared;
    let mut queue = lock.lock().ok()?;
    loop {
        if queue.closed {
            return None;
        }
        if let Some(task) = queue.tasks.pop_front() {
            queue.busy += 1;
            return Some(task);
        }
        queue = wake.wait(queue).ok()?;
    }
}

fn finish_task(shared: &SharedQueue) {
    if let Ok(mut queue) = shared.0.lock() {
        queue.busy = queue.busy.saturating_sub(1);
    }
}

#[cfg(test)]
mod tests;
