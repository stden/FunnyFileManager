//! Purpose: Retain private terminal control while a Unix capture reader borrows its fd.

use portable_pty::{MasterPty, PtySize};
use std::{
    io,
    sync::{Arc, Mutex, MutexGuard},
};

#[derive(Clone)]
pub(super) struct Master(Arc<Mutex<Box<dyn MasterPty + Send>>>);

impl Master {
    pub fn new(master: Box<dyn MasterPty + Send>) -> Self {
        Self(Arc::new(Mutex::new(master)))
    }

    pub fn size(&self) -> io::Result<PtySize> {
        self.lock().get_size().map_err(io::Error::other)
    }

    pub fn resize(&self, size: PtySize) -> io::Result<()> {
        self.lock().resize(size).map_err(io::Error::other)
    }

    #[cfg(unix)]
    pub fn foreground_stop(&self) -> io::Result<()> {
        crate::platform::kill_terminal_foreground(&**self.lock())
    }

    #[cfg(unix)]
    pub fn nonblocking_fd(&self) -> io::Result<std::os::fd::RawFd> {
        crate::platform::nonblocking_terminal_fd(&**self.lock())
    }

    fn lock(&self) -> MutexGuard<'_, Box<dyn MasterPty + Send>> {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

#[cfg(all(test, unix))]
mod tests;
