//! Purpose: Unix process identity, access checks, signals, renice and the
//! kernel's page size and clock rate, through `rustix`'s safe system-call
//! wrappers rather than raw `libc` calls.
//!
//! The process calls take the validated [`Pid`], [`Signal`] and [`Niceness`]
//! of the models: this file holds the one conversion of each to the system's
//! form, so no other module can build a raw PID or a signal number.

use std::io;
#[cfg(target_os = "linux")]
use std::os::fd::OwnedFd;
use std::path::Path;

use rustix::fs::{Access, AtFlags, CWD};
use rustix::process::Signal as SystemSignal;

use crate::models::{Niceness, Pid, Signal};

mod group;
pub use group::{hang_up_process_group, has_exited, kill_process_group};

/// The effective user id: the one the kernel checks file access against, and
/// the owner of every file this process creates.
#[must_use]
pub fn effective_uid() -> u32 {
    rustix::process::geteuid().as_raw()
}

/// The real user id: who started the process, whatever it has been granted.
#[must_use]
pub fn real_uid() -> u32 {
    rustix::process::getuid().as_raw()
}

/// Whether this process runs as root (effective uid 0), and so can overwrite
/// anything — which is what a `#` prompt means.
#[must_use]
pub fn is_root() -> bool {
    effective_uid() == 0
}

/// Ask the kernel whether this process may write `path`, as its *effective*
/// user: `faccessat(2)` with `AT_EACCESS`.
///
/// Only the kernel accounts for group membership, POSIX ACLs, a read-only
/// mount and root's `CAP_DAC_OVERRIDE`, so this is the real answer where the
/// permission bits are only a guess.
///
/// # Errors
///
/// The kernel's refusal (`EACCES`, `EPERM`, `EROFS`, `ENOENT`, …) with its
/// raw errno intact; a path with an interior NUL is `EINVAL`.
pub fn check_write_access(path: &Path) -> io::Result<()> {
    rustix::fs::accessat(CWD, path, Access::WRITE_OK, AtFlags::EACCESS).map_err(io::Error::from)
}

/// The size of a memory page in bytes: 4 KiB on x86-64, 16 or 64 KiB on some
/// arm64 kernels, so it is asked, never assumed.
#[must_use]
pub fn page_size() -> u64 {
    // No target Rust supports has a `usize` wider than 64 bits. Were one to
    // appear, 0 is the value the process reader already rejects.
    u64::try_from(rustix::param::page_size()).unwrap_or(0)
}

/// Clock ticks per second (`USER_HZ`), the unit of the CPU times in `/proc`.
#[must_use]
pub fn clock_ticks_per_second() -> u64 {
    rustix::param::clock_ticks_per_second()
}

/// Sends `signal` to the process `pid` with `kill(2)`.
///
/// Only that one process is signalled: a [`Pid`] cannot be 0, −1 or a group.
/// Nothing pins it, so between the caller's identity check and this call the
/// process may exit and its PID go to a newcomer; where it can, the caller
/// signals through a [`ProcessHandle`] instead.
///
/// # Errors
///
/// The kernel's refusal: `ESRCH` when the process is gone, `EPERM` when it
/// belongs to another user.
#[cfg_attr(
    not(any(target_os = "linux", test)),
    expect(
        dead_code,
        reason = "only the Linux process table calls it; the other Unix systems have no reader yet"
    )
)]
pub fn signal_process(pid: Pid, signal: Signal) -> io::Result<()> {
    rustix::process::kill_process(rustix_pid(pid)?, rustix_signal(signal)).map_err(io::Error::from)
}

/// Sets the nice value of the process `pid` with `setpriority(PRIO_PROCESS)`.
///
/// Always names the process: `setpriority` with no id would renice this
/// program. On Linux that renices the thread whose id is `pid`, as
/// `renice(1)` does.
///
/// # Errors
///
/// The kernel's refusal: `ESRCH` when the process is gone; `EPERM` for another
/// user's process; `EACCES` when lowering nice needs `CAP_SYS_NICE`.
#[cfg_attr(
    not(any(target_os = "linux", test)),
    expect(
        dead_code,
        reason = "only the Linux process table calls it; the other Unix systems have no reader yet"
    )
)]
pub fn set_niceness(pid: Pid, nice: Niceness) -> io::Result<()> {
    rustix::process::setpriority_process(Some(rustix_pid(pid)?), i32::from(nice.get()))
        .map_err(io::Error::from)
}

/// A pidfd: a handle that names one process for as long as it is open.
///
/// A signal sent through it can only reach that process. Once it has exited,
/// the kernel answers `ESRCH` even if its PID has already gone to a newcomer.
#[cfg(target_os = "linux")]
#[derive(Debug)]
pub struct ProcessHandle(OwnedFd);

#[cfg(target_os = "linux")]
impl ProcessHandle {
    /// Pins the process `pid` with `pidfd_open(2)`.
    ///
    /// # Errors
    ///
    /// `ESRCH` when there is no such process; `ENOSYS` before Linux 5.3 and
    /// `EPERM` under a seccomp filter, where the caller falls back to
    /// [`signal_process`].
    pub fn open(pid: Pid) -> io::Result<Self> {
        rustix::process::pidfd_open(rustix_pid(pid)?, rustix::process::PidfdFlags::empty())
            .map(Self)
            .map_err(io::Error::from)
    }

    /// Sends `signal` to the pinned process with `pidfd_send_signal(2)`.
    ///
    /// # Errors
    ///
    /// `ESRCH` once the process has exited, `EPERM` for another user's process.
    pub fn signal(&self, signal: Signal) -> io::Result<()> {
        rustix::process::pidfd_send_signal(&self.0, rustix_signal(signal)).map_err(io::Error::from)
    }
}

/// The one conversion of a [`Pid`] to the system's form. A `Pid` is never
/// zero or negative, so the `InvalidInput` answer is only a guard.
fn rustix_pid(pid: Pid) -> io::Result<rustix::process::Pid> {
    rustix::process::Pid::from_raw(pid.raw())
        .ok_or_else(|| io::Error::from(io::ErrorKind::InvalidInput))
}

/// The system's number for `signal`: the one place signal numbers, which
/// differ between Unix systems, are chosen.
const fn rustix_signal(signal: Signal) -> SystemSignal {
    match signal {
        Signal::Terminate => SystemSignal::TERM,
        Signal::Kill => SystemSignal::KILL,
        Signal::Hangup => SystemSignal::HUP,
        Signal::Interrupt => SystemSignal::INT,
        Signal::Quit => SystemSignal::QUIT,
        Signal::Abort => SystemSignal::ABORT,
        Signal::Stop => SystemSignal::STOP,
        Signal::Continue => SystemSignal::CONT,
        Signal::TerminalStop => SystemSignal::TSTP,
        Signal::User1 => SystemSignal::USR1,
        Signal::User2 => SystemSignal::USR2,
        Signal::Alarm => SystemSignal::ALARM,
        Signal::WindowChange => SystemSignal::WINCH,
    }
}

#[cfg(test)]
mod tests;
