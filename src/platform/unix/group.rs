//! Purpose: Signal the process group a child leads, and see whether the child
//! has exited without reaping it, so its id keeps naming that group.

use std::io;
use std::process::Child;

use rustix::process::{Pid as SystemPid, Signal as SystemSignal};
use rustix::process::{WaitId, WaitIdOptions};

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
    signal_group(leader, SystemSignal::KILL)
}

/// SIGHUP every process in the group `leader` leads: what a shell sends its
/// jobs when its terminal goes away. The same conditions as
/// [`kill_process_group`].
///
/// # Errors
///
/// The kernel's refusal, `ESRCH` when there is no such group.
pub fn hang_up_process_group(leader: &Child) -> io::Result<()> {
    signal_group(leader, SystemSignal::HUP)
}

/// Whether `child` has exited. It is left unreaped, a zombie: until
/// `Child::wait` reaps it, its id cannot go to another process, so it still
/// names only the group `child` led.
///
/// # Errors
///
/// The kernel's refusal, `ECHILD` once the child has been reaped.
pub fn has_exited(child: &Child) -> io::Result<bool> {
    let exited = rustix::process::waitid(
        WaitId::Pid(SystemPid::from_child(child)),
        WaitIdOptions::EXITED | WaitIdOptions::NOHANG | WaitIdOptions::NOWAIT,
    )?;
    Ok(exited.is_some())
}

fn signal_group(leader: &Child, signal: SystemSignal) -> io::Result<()> {
    rustix::process::kill_process_group(SystemPid::from_child(leader), signal)
        .map_err(io::Error::from)
}

/// Stop the foreground job of an owned private terminal, including a shell's
/// job-control group, whose identity comes from that terminal's kernel state.
pub fn kill_terminal_foreground(master: &dyn portable_pty::MasterPty) -> io::Result<()> {
    let Some(pid) = master.process_group_leader().and_then(SystemPid::from_raw) else {
        return Ok(());
    };
    match rustix::process::kill_process_group(pid, SystemSignal::KILL) {
        Err(rustix::io::Errno::SRCH) => Ok(()),
        result => result.map_err(io::Error::from),
    }
}
