//! Purpose: The adapters that read a platform's process table into the
//! models' ports. Linux reads `/proc` with the standard library; the other
//! platforms keep the text report until they have a reader of their own.
//!
//! The Linux reader is compiled on Linux, and in tests everywhere, so every
//! CI runner checks it against fixture text.

#[cfg(any(target_os = "linux", test))]
mod procfs;

use std::sync::Arc;

use crate::models::ProcessHost;
#[cfg(target_os = "linux")]
use crate::models::{ControlError, PriorityChange, ProcessKey, ProcessReader, SampleError, Signal};

/// This system's process table, or `None` where FFM has no reader for it
/// yet: there the task list falls back to the `ps` or `tasklist` report.
#[cfg(target_os = "linux")]
pub fn system_host() -> Option<Arc<dyn ProcessHost>> {
    let host: Arc<dyn ProcessHost> = Arc::new(SystemHost(procfs::ProcessControl::system()?));
    Some(host)
}

/// This system's process table, or `None` where FFM has no reader for it
/// yet: there the task list falls back to the `ps` or `tasklist` report.
#[cfg(not(target_os = "linux"))]
pub const fn system_host() -> Option<Arc<dyn ProcessHost>> {
    None
}

/// Linux's process table: `/proc`, read by one [`procfs::ProcFs`] per
/// task-list session, and its processes signalled and reniced only after
/// their identity is checked.
#[cfg(target_os = "linux")]
struct SystemHost(procfs::ProcessControl);

#[cfg(target_os = "linux")]
impl ProcessHost for SystemHost {
    fn reader(&self) -> Result<Box<dyn ProcessReader>, SampleError> {
        Ok(Box::new(procfs::ProcFs::system()?))
    }

    fn signal(&self, key: ProcessKey, signal: Signal) -> Result<(), ControlError> {
        self.0.signal(key, signal)
    }

    fn set_priority(&self, key: ProcessKey, change: PriorityChange) -> Result<(), ControlError> {
        self.0.set_priority(key, change)
    }
}

#[cfg(test)]
mod tests;
