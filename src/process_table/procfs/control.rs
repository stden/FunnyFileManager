//! Purpose: Signal and renice a process only while it is still the process
//! the user chose: pinned with a pidfd where the kernel has them, its start
//! time re-read from `/proc` and compared with the one the table showed, and
//! FFM itself, init and kernel threads refused whatever the caller checked.
//!
//! A PID is reused once its process is reaped, so a PID alone can name a
//! newcomer by the time Enter is pressed. A pidfd opened before the re-read
//! names the process the re-read then checked: if it exits, the kernel
//! answers `ESRCH` rather than signal whoever holds the PID next. Without
//! pidfds (Linux before 5.3, or a seccomp filter) and for renice, which has
//! no pidfd form, a window of microseconds remains between check and call.
//!
//! The decisions — how an open, a re-read and an errno are read — are pure
//! and compiled on every CI system; the system calls are Linux only.

use std::io;
#[cfg(target_os = "linux")]
use std::path::PathBuf;

use crate::models::{ControlError, StartTime};
#[cfg(target_os = "linux")]
use crate::models::{ControlRefusal, Pid, PriorityChange, ProcessKey, Signal, Start};
#[cfg(target_os = "linux")]
use crate::platform::{self, ProcessHandle};

use super::files::ESRCH;
#[cfg(target_os = "linux")]
use super::files::{Buffer, ProcessFile};
use super::parse::Stat;
#[cfg(target_os = "linux")]
use super::parse::{self, ClockTicks};
#[cfg(target_os = "linux")]
use super::{system_ticks, PROC};

/// Linux's `EPERM`, `EACCES` and `ENOSYS`, named with the Linux values so the
/// mapping compiles and is tested on every CI system.
const EPERM: i32 = 1;
const EACCES: i32 = 13;
const ENOSYS: i32 = 38;

/// How a process is held while it is checked and signalled.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Pin<H> {
    /// Through a pidfd: a signal can reach only that process.
    Pinned(H),
    /// By PID alone: the kernel has no pidfds, or a seccomp filter refuses
    /// them.
    Unpinned,
}

/// What opening a pidfd means for the signal: a pin, no pin to be had, or
/// no process to pin.
pub(super) fn classify_open<H>(opened: io::Result<H>) -> Result<Pin<H>, ControlError> {
    match opened {
        Ok(handle) => Ok(Pin::Pinned(handle)),
        Err(error) => match error.raw_os_error() {
            Some(ENOSYS | EPERM) => Ok(Pin::Unpinned),
            Some(ESRCH) => Err(ControlError::Gone),
            _ => Err(ControlError::Failed(error.kind())),
        },
    }
}

/// The re-read `stat` of the process, if it is still the one that started
/// at `expected`: gone when the file is, replaced when the start differs,
/// and unverifiable when the line cannot be parsed.
pub(super) fn verify_start(
    read: io::Result<Option<Stat>>,
    expected: StartTime,
) -> Result<Stat, ControlError> {
    match read {
        Err(error) if error.kind() == io::ErrorKind::NotFound => Err(ControlError::Gone),
        Err(error) => Err(control_error(&error)),
        Ok(None) => Err(ControlError::Unverifiable),
        Ok(Some(stat)) if stat.start != expected => Err(ControlError::Replaced),
        Ok(Some(stat)) => Ok(stat),
    }
}

/// What the kernel's refusal of a signal, a renice or a read means.
pub(super) fn control_error(error: &io::Error) -> ControlError {
    match error.raw_os_error() {
        Some(ESRCH) => ControlError::Gone,
        Some(EPERM | EACCES) => ControlError::PermissionDenied,
        _ => ControlError::Failed(error.kind()),
    }
}

/// Signals and renices through `/proc`, knowing which process is FFM.
#[cfg(target_os = "linux")]
pub(in crate::process_table) struct ProcessControl {
    root: PathBuf,
    own: Pid,
    ticks: ClockTicks,
}

#[cfg(target_os = "linux")]
impl ProcessControl {
    /// Control of this system's processes, run from this process; `None`
    /// should the kernel report a PID or a clock rate no process can have.
    pub(in crate::process_table) fn system() -> Option<Self> {
        Some(Self::new(
            PathBuf::from(PROC),
            Pid::new(std::process::id())?,
            system_ticks()?,
        ))
    }

    /// Control reading identities under `root`, refusing to signal `own`.
    pub(super) fn new(root: PathBuf, own: Pid, ticks: ClockTicks) -> Self {
        Self { root, own, ticks }
    }

    /// Sends `signal` to the process `key` names, if it is still that
    /// process and may be signalled at all.
    pub(in crate::process_table) fn signal(
        &self,
        key: ProcessKey,
        signal: Signal,
    ) -> Result<(), ControlError> {
        let expected = known_start(key)?;
        let pid = key.pid();
        refuse(ControlRefusal::signal_to_pid(pid, Some(self.own)))?;
        let pin = classify_open(ProcessHandle::open(pid))?;
        let stat = verify_start(self.reread(pid), expected)?;
        refuse(ControlRefusal::signal_to_kind(stat.kind))?;
        send(pin, pid, signal).map_err(|error| control_error(&error))
    }

    /// Sets the nice value of the process `key` names, if it is still that
    /// process. Renicing FFM itself, init or a kernel thread is harmless and
    /// allowed; a Windows priority class is not a Linux thing.
    pub(in crate::process_table) fn set_priority(
        &self,
        key: ProcessKey,
        change: PriorityChange,
    ) -> Result<(), ControlError> {
        let nice = match change {
            PriorityChange::Nice(nice) => nice,
            PriorityChange::Class(_) => return Err(ControlError::Unsupported),
        };
        let expected = known_start(key)?;
        verify_start(self.reread(key.pid()), expected)?;
        platform::set_niceness(key.pid(), nice).map_err(|error| control_error(&error))
    }

    /// The `stat` of `pid` as it is now.
    fn reread(&self, pid: Pid) -> io::Result<Option<Stat>> {
        let path = self
            .root
            .join(pid.to_string())
            .join(ProcessFile::Stat.name());
        let mut buffer = Buffer::default();
        let text = buffer.read(&path, ProcessFile::Stat.cap())?;
        Ok(parse::stat(&text, pid, self.ticks))
    }
}

/// When the process `key` names started, or unverifiable when its source
/// could not say.
#[cfg(target_os = "linux")]
const fn known_start(key: ProcessKey) -> Result<StartTime, ControlError> {
    match key.start() {
        Start::Known(start) => Ok(start),
        Start::Unknown => Err(ControlError::Unverifiable),
    }
}

/// `refusal` as the error it ends the request with.
#[cfg(target_os = "linux")]
fn refuse(refusal: Option<ControlRefusal>) -> Result<(), ControlError> {
    refusal.map_or(Ok(()), |refusal| Err(ControlError::Refused(refusal)))
}

/// Sends `signal` through the pin, or by PID where there is none.
#[cfg(target_os = "linux")]
pub(super) fn send(pin: Pin<ProcessHandle>, pid: Pid, signal: Signal) -> io::Result<()> {
    match pin {
        Pin::Pinned(handle) => handle.signal(signal),
        Pin::Unpinned => platform::signal_process(pid, signal),
    }
}
