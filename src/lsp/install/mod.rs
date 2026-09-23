//! Purpose: Cancellable installation jobs owned by the LSP worker, never by the UI thread.

use crate::config::ServerSettings;
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    },
    thread,
};

mod archive;
mod environment;
mod error;
mod launch;
mod recipes;
mod report;
pub use report::inventory;
mod runner;
mod storage;
pub(super) use error::InstallError;

pub(super) struct Job {
    stop: Arc<AtomicBool>,
    result: mpsc::Receiver<Result<ServerSettings, InstallError>>,
    thread: Option<thread::JoinHandle<()>>,
}

impl Job {
    pub fn start(
        settings: ServerSettings,
        cache: PathBuf,
        workspace: PathBuf,
    ) -> Result<Self, InstallError> {
        let stop = Arc::new(AtomicBool::new(false));
        let cancelled = Arc::clone(&stop);
        let (sender, result) = mpsc::sync_channel(1);
        let thread = thread::Builder::new()
            .name("ffm-lsp-install".into())
            .spawn(move || {
                let installed = storage::prepare(&settings, &cache, &workspace, &cancelled);
                // Closing the editor deliberately discards the installation result.
                let _ = sender.send(installed);
            })?;
        Ok(Self {
            stop,
            result,
            thread: Some(thread),
        })
    }

    pub fn poll(&self) -> Option<Result<ServerSettings, InstallError>> {
        match self.result.try_recv() {
            Ok(result) => Some(result),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => Some(Err(InstallError::Worker)),
        }
    }
}

impl Drop for Job {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if self
            .thread
            .take()
            .is_some_and(|thread| thread.join().is_err())
        {
            crate::diagnostics::log_error("LSP installer task panicked");
        }
    }
}

pub(super) fn cache_dir() -> Option<PathBuf> {
    crate::config::dir().map(|root| root.join("lsp"))
}

fn check_cancel(stop: &AtomicBool) -> Result<(), InstallError> {
    if stop.load(Ordering::Acquire) {
        Err(InstallError::Cancelled)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
#[path = "tests_fixtures.rs"]
mod fixtures;

#[cfg(test)]
#[path = "tests_packages.rs"]
mod package_tests;

#[cfg(test)]
#[path = "tests_smoke.rs"]
mod smoke_tests;
