//! Purpose: A language server's process tree on each platform — its own group
//! or job, the exit check, termination and reaping.

use std::process::{Child, Command};

#[cfg(windows)]
pub(in crate::lsp) fn configure(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW: only the editor owns a terminal.
}

#[cfg(unix)]
pub(in crate::lsp) fn configure(command: &mut Command) {
    use std::os::unix::process::CommandExt;
    command.process_group(0);
}

#[cfg(unix)]
pub(in crate::lsp) fn exited(child: &mut Child) -> bool {
    use rustix::process::{waitid, Pid, WaitId, WaitIdOptions};
    // NOWAIT retains the leader's PID until terminate has killed its entire group.
    waitid(
        WaitId::Pid(Pid::from_child(child)),
        WaitIdOptions::EXITED | WaitIdOptions::NOHANG | WaitIdOptions::NOWAIT,
    )
    .is_ok_and(|status| status.is_some())
}

#[cfg(windows)]
pub(in crate::lsp) fn exited(child: &mut Child) -> bool {
    child.try_wait().is_ok_and(|status| status.is_some())
}

#[cfg(unix)]
pub(in crate::lsp) fn terminate(child: &mut Child) {
    // The leader has not been reaped: its PID cannot belong to an unrelated group.
    // A group that is already gone needs no signal, and `reap` kills the
    // leader either way.
    drop(crate::platform::kill_process_group(child));
    reap(child);
}

#[cfg(windows)]
pub(in crate::lsp) fn terminate(child: &mut Child) {
    reap(child);
}

fn reap(child: &mut Child) {
    if !child.try_wait().is_ok_and(|status| status.is_some()) {
        // The job/group may have terminated the process between try_wait and kill.
        drop(child.kill());
    }
    if child.wait().is_err() {
        crate::diagnostics::log_error("LSP process reap failed");
    }
}
