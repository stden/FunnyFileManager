//! Purpose: Unix process identity, access checks and group kills, through
//! `rustix`'s safe system-call wrappers rather than raw `libc` calls.

use std::io;
use std::path::Path;
use std::process::Child;

use rustix::fs::{Access, AtFlags, CWD};
use rustix::process::{Pid, Signal};

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

/// SIGKILL every process in the group `leader` leads.
///
/// `leader` must have been spawned as a group leader
/// (`CommandExt::process_group(0)`) and not yet reaped. Any other child leads
/// no group, so the kernel answers `ESRCH` and nothing is signalled.
///
/// # Errors
///
/// The kernel's refusal, `ESRCH` when there is no such group.
pub fn kill_process_group(leader: &Child) -> io::Result<()> {
    rustix::process::kill_process_group(Pid::from_child(leader), Signal::KILL)
        .map_err(io::Error::from)
}
