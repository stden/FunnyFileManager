//! Purpose: The adapters that read a platform's process and network tables into
//! the models' ports. Linux reads `/proc` and `/sys/class/net`; Windows queries
//! Win32 Toolhelp32, `GetSystemTimes` and IP Helper; other platforms keep the
//! text report until they have a reader of their own.

#[cfg(any(target_os = "linux", windows, test))]
mod net_probe;
#[cfg(any(target_os = "linux", test))]
mod procfs;
#[cfg(windows)]
mod windows;

use std::sync::Arc;

use crate::models::ProcessHost;
#[cfg(target_os = "linux")]
use crate::models::{
    ControlError, NetProber, PriorityChange, ProcessKey, ProcessReader, SampleError, Signal,
};

/// This system's live process and network host on Linux.
#[cfg(target_os = "linux")]
pub fn system_host() -> Option<Arc<dyn ProcessHost>> {
    let host: Arc<dyn ProcessHost> = Arc::new(SystemHost(procfs::ProcessControl::system()?));
    Some(host)
}

/// This system's live process and network host on Windows.
#[cfg(windows)]
pub fn system_host() -> Option<Arc<dyn ProcessHost>> {
    let host: Arc<dyn ProcessHost> = Arc::new(windows::WindowsHost::system()?);
    Some(host)
}

/// Fallback where FFM has no native reader yet (`ps` report).
#[cfg(not(any(target_os = "linux", windows)))]
pub const fn system_host() -> Option<Arc<dyn ProcessHost>> {
    None
}

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

    fn net_prober(&self) -> Option<Box<dyn NetProber>> {
        Some(Box::new(net_probe::SystemNetProber::default()))
    }
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_net_probe;
