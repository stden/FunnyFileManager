//! Purpose: Bounded background worker queue for uncached panel `git check-ignore` probes.

use std::collections::{BTreeSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc, Condvar, Mutex};

use super::super::Redraw;
use crate::models::{App, FileName, IgnoredCandidate};

const MAX_IGNORED_WORKERS: usize = 2;

type IgnoredTask = (PathBuf, Vec<IgnoredCandidate>);
type IgnoredOutcome = (PathBuf, BTreeSet<FileName>);
pub(super) type IgnoredProber =
    Arc<dyn Fn(&Path, &[IgnoredCandidate]) -> BTreeSet<FileName> + Send + Sync>;

#[derive(Default)]
pub(super) struct Queue {
    tasks: VecDeque<IgnoredTask>,
    busy: usize,
    closed: bool,
}

type SharedQueue = Arc<(Mutex<Queue>, Condvar)>;

#[cfg(any(test, tarpaulin))]
thread_local! {
    pub(super) static FORCE_IGNORED_SPAWN_FAILURE: std::cell::Cell<bool> =
        const { std::cell::Cell::new(false) };
}

pub(super) struct IgnoredPool {
    pub(super) shared: SharedQueue,
    outcomes: mpsc::Receiver<IgnoredOutcome>,
    sender: mpsc::Sender<IgnoredOutcome>,
    workers: usize,
}

pub(super) struct GitIgnoredJobs {
    prober: IgnoredProber,
    max_workers: usize,
    pool: Option<IgnoredPool>,
}

impl Default for GitIgnoredJobs {
    fn default() -> Self {
        Self {
            prober: Arc::new(crate::git::probe_ignored),
            max_workers: MAX_IGNORED_WORKERS,
            pool: None,
        }
    }
}

impl Drop for GitIgnoredJobs {
    fn drop(&mut self) {
        if let Some(pool) = &self.pool {
            let (lock, wake) = &*pool.shared;
            if let Ok(mut queue) = lock.lock() {
                queue.tasks.clear();
                queue.closed = true;
                wake.notify_all();
            }
        }
    }
}

impl GitIgnoredJobs {
    #[cfg(test)]
    pub(super) fn with_workers<F>(max_workers: usize, prober: F) -> Self
    where
        F: Fn(&Path, &[IgnoredCandidate]) -> BTreeSet<FileName> + Send + Sync + 'static,
    {
        Self {
            prober: Arc::new(prober),
            max_workers: max_workers.max(1),
            pool: None,
        }
    }

    pub(super) fn pump(&mut self, app: &mut App) -> Redraw {
        let left_batch = app.left_panel.take_ignored_probe();
        let right_batch = app.right_panel.take_ignored_probe();
        if self.pool.is_none() && left_batch.is_none() && right_batch.is_none() {
            return Redraw::Unchanged;
        }
        let active = [
            app.left_panel.path.as_path(),
            app.right_panel.path.as_path(),
        ];
        let max_workers = self.max_workers;
        let pool = self.pool.get_or_insert_with(IgnoredPool::new);
        let demand = pool.update_queue(active, [left_batch, right_batch]);
        pool.grow_to(demand.min(max_workers), &self.prober);
        pool.collect(app)
    }
}

impl IgnoredPool {
    pub(super) fn new() -> Self {
        let (sender, outcomes) = mpsc::channel();
        Self {
            shared: Arc::new((Mutex::new(Queue::default()), Condvar::new())),
            outcomes,
            sender,
            workers: 0,
        }
    }

    pub(super) fn update_queue(
        &self,
        active: [&Path; 2],
        batches: [Option<IgnoredTask>; 2],
    ) -> usize {
        let (lock, wake) = &*self.shared;
        let Ok(mut queue) = lock.lock() else {
            return 0;
        };
        queue
            .tasks
            .retain(|(dir, _)| active.contains(&dir.as_path()));
        let mut added = false;
        for (dir, candidates) in batches.into_iter().flatten() {
            if let Some(slot) = queue.tasks.iter_mut().find(|(queued, _)| *queued == dir) {
                slot.1 = candidates;
            } else {
                queue.tasks.push_back((dir, candidates));
                added = true;
            }
        }
        let demand = queue.tasks.len().saturating_add(queue.busy);
        drop(queue);
        if added {
            wake.notify_all();
        }
        demand
    }

    fn grow_to(&mut self, target: usize, prober: &IgnoredProber) {
        while self.workers < target {
            let shared = Arc::clone(&self.shared);
            if spawn_worker(shared, self.sender.clone(), Arc::clone(prober)).is_err() {
                break;
            }
            self.workers += 1;
        }
    }

    fn collect(&self, app: &mut App) -> Redraw {
        let mut redraw = Redraw::Unchanged;
        while let Ok((dir, ignored)) = self.outcomes.try_recv() {
            let left = app.left_panel.apply_ignored_probe(&dir, &ignored);
            let right = app.right_panel.apply_ignored_probe(&dir, &ignored);
            if left || right {
                redraw = Redraw::Needed;
            }
        }
        redraw
    }
}

fn spawn_worker(
    shared: SharedQueue,
    sender: mpsc::Sender<IgnoredOutcome>,
    prober: IgnoredProber,
) -> std::io::Result<std::thread::JoinHandle<()>> {
    #[cfg(any(test, tarpaulin))]
    if FORCE_IGNORED_SPAWN_FAILURE.get() {
        return Err(std::io::Error::other("forced ignored worker spawn failure"));
    }
    std::thread::Builder::new()
        .name("ffm-git-ignore".to_owned())
        .spawn(move || {
            while let Some((dir, candidates)) = next_task(&shared) {
                let ignored = prober(&dir, &candidates);
                if let Ok(mut queue) = shared.0.lock() {
                    queue.busy = queue.busy.saturating_sub(1);
                }
                drop(sender.send((dir, ignored)));
            }
        })
}

fn next_task(shared: &SharedQueue) -> Option<IgnoredTask> {
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
