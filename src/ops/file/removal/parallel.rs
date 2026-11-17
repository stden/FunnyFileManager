//! Purpose: Parallel directory tree removal: concurrent folder discovery and
//! batched file unlinking in Stage 1, followed by deepest-first directory
//! removal in Stage 2, with streaming progress and cancellation on the caller.

use std::cmp::Reverse;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Condvar, Mutex, MutexGuard, PoisonError};
use std::time::Duration;
use std::{fs, io};

#[cfg(any(test, tarpaulin))]
use super::RemovalStep;

mod worker;

pub(super) const BATCH_SIZE: usize = 16;
const POLL_SLICE: Duration = Duration::from_millis(15);

pub(super) enum Task {
    ScanDir { path: PathBuf, depth: usize },
    UnlinkFiles(Vec<(PathBuf, fs::FileType)>),
    RemoveDirs(Vec<PathBuf>),
}

#[derive(Default)]
pub(super) struct Queue {
    pub(super) tasks: Vec<Task>,
    pub(super) active: usize,
    pub(super) dirs: Vec<(usize, PathBuf)>,
    pub(super) discovered: u64,
    pub(super) removed: u64,
    pub(super) current: Option<PathBuf>,
    pub(super) error: Option<io::Error>,
}

struct ProgressStep {
    discovered: u64,
    removed: u64,
    current: Option<PathBuf>,
    idle: bool,
    stopped: bool,
}

pub(super) struct Coordinator {
    mutex: Mutex<Queue>,
    pub(super) work_cv: Condvar,
    pub(super) main_cv: Condvar,
    pub(super) stop: AtomicBool,
    #[cfg(any(test, tarpaulin))]
    pub(super) forced: Option<RemovalStep>,
}

pub(super) fn clamp_workers(available: Option<std::num::NonZeroUsize>) -> usize {
    available.map_or(4, |count| count.get().clamp(2, 12))
}

fn worker_count() -> usize {
    clamp_workers(std::thread::available_parallelism().ok())
}

pub(super) fn remove_tree(root: &Path, cancel: &mut dyn FnMut(bool) -> bool) -> io::Result<()> {
    crate::progress::entering(root);
    let coordinator = Coordinator::new(
        root,
        #[cfg(any(test, tarpaulin))]
        super::FORCE_REMOVAL_FAILURE.get(),
    );
    let mut painted = false;
    std::thread::scope(|scope| {
        for _ in 0..worker_count() {
            scope.spawn(|| coordinator.worker_loop());
        }
        let result = coordinator.run_stages(cancel, &mut painted);
        coordinator.stop_workers();
        result
    })
}

impl Coordinator {
    fn new(root: &Path, #[cfg(any(test, tarpaulin))] forced: Option<RemovalStep>) -> Self {
        let initial = root.to_path_buf();
        Self {
            mutex: Mutex::new(Queue {
                tasks: vec![Task::ScanDir {
                    path: initial.clone(),
                    depth: 0,
                }],
                dirs: vec![(0, initial)],
                ..Queue::default()
            }),
            work_cv: Condvar::new(),
            main_cv: Condvar::new(),
            stop: AtomicBool::new(false),
            #[cfg(any(test, tarpaulin))]
            forced,
        }
    }

    pub(super) fn is_stopped(&self) -> bool {
        self.stop.load(Ordering::Relaxed)
    }

    pub(super) fn lock(&self) -> MutexGuard<'_, Queue> {
        self.mutex.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn stop_workers(&self) {
        {
            let _guard = self.lock();
            self.stop.store(true, Ordering::Release);
        }
        self.work_cv.notify_all();
    }

    fn run_stages(
        &self,
        cancel: &mut dyn FnMut(bool) -> bool,
        painted: &mut bool,
    ) -> io::Result<()> {
        self.drive_stage(cancel, painted)?;
        let mut dirs = std::mem::take(&mut self.lock().dirs);
        dirs.sort_unstable_by_key(|(depth, _)| Reverse(*depth));
        for level in dirs.chunk_by(|left, right| left.0 == right.0) {
            self.enqueue_dir_level(level);
            self.drive_stage(cancel, painted)?;
        }
        Ok(())
    }

    fn enqueue_dir_level(&self, level: &[(usize, PathBuf)]) {
        {
            let mut queue = self.lock();
            for chunk in level.chunks(BATCH_SIZE) {
                let paths = chunk.iter().map(|(_, path)| path.clone()).collect();
                queue.tasks.push(Task::RemoveDirs(paths));
            }
        }
        self.work_cv.notify_all();
    }

    fn drive_stage(
        &self,
        cancel: &mut dyn FnMut(bool) -> bool,
        painted: &mut bool,
    ) -> io::Result<()> {
        loop {
            let step = self.take_progress_or_wait();
            *painted = crate::progress::delete_progress(
                step.discovered,
                step.removed,
                step.current.as_deref(),
            );
            if step.stopped {
                return Err(self.take_error());
            }
            if cancel(*painted) {
                self.stop_workers();
                return Err(io::Error::new(io::ErrorKind::Interrupted, "cancelled"));
            }
            if step.idle {
                return Ok(());
            }
        }
    }

    fn take_progress_or_wait(&self) -> ProgressStep {
        let mut queue = self.lock();
        if queue.discovered == 0
            && queue.removed == 0
            && !self.stop.load(Ordering::Acquire)
            && !(queue.active == 0 && queue.tasks.is_empty())
        {
            let (next, _) = self
                .main_cv
                .wait_timeout(queue, POLL_SLICE)
                .unwrap_or_else(PoisonError::into_inner);
            queue = next;
        }
        ProgressStep {
            discovered: std::mem::take(&mut queue.discovered),
            removed: std::mem::take(&mut queue.removed),
            current: queue.current.take(),
            idle: queue.active == 0 && queue.tasks.is_empty(),
            stopped: self.stop.load(Ordering::Acquire),
        }
    }

    fn take_error(&self) -> io::Error {
        let fallback = io::Error::last_os_error;
        self.lock().error.take().unwrap_or_else(fallback)
    }
}

#[cfg(test)]
mod tests;
