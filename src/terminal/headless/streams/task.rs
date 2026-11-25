//! Purpose: Stop and join Unix capture workers before releasing their retained master.

use std::{
    io,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
};

pub(super) struct Task {
    stop: Arc<AtomicBool>,
    #[cfg(unix)]
    worker: Option<thread::JoinHandle<()>>,
}

impl Task {
    pub fn spawn(run: impl FnOnce(Arc<AtomicBool>) + Send + 'static) -> io::Result<Self> {
        let stop = Arc::new(AtomicBool::new(false));
        let stopped = Arc::clone(&stop);
        let worker = thread::Builder::new()
            .name("ffm-private-output".to_owned())
            .spawn(move || run(stopped))?;
        #[cfg(not(unix))]
        drop(worker);
        Ok(Self {
            stop,
            #[cfg(unix)]
            worker: Some(worker),
        })
    }

    #[cfg_attr(
        not(unix),
        expect(
            clippy::needless_pass_by_ref_mut,
            reason = "worker.take() on Unix requires &mut self; the field is absent on Windows where the lint fires"
        )
    )]
    pub fn close(&mut self) {
        self.stop.store(true, Ordering::Release);
        #[cfg(unix)]
        if let Some(worker) = self.worker.take() {
            if worker.join().is_err() {
                crate::diagnostics::log_error("Private terminal reader panicked");
            }
        }
    }
}

impl Drop for Task {
    fn drop(&mut self) {
        self.close();
    }
}

#[cfg(all(test, unix))]
mod tests;
