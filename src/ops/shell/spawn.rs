//! Purpose: Start the platform shell with piped output in a process group of
//! its own, and tear the whole group down when it has to stop.

use std::process::{Child, Command, Stdio};
#[cfg(unix)]
use std::time::{Duration, Instant};
use std::{io, path::Path};

#[cfg(windows)]
use super::windows_shell_args;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[cfg(windows)]
thread_local! {
    static ACTIVE_JOBS: std::cell::RefCell<Vec<(u32, std::os::windows::io::OwnedHandle)>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

pub(super) fn spawn_shell(command: &str, current_dir: &Path) -> io::Result<Child> {
    #[cfg(windows)]
    let mut shell = {
        use std::os::windows::process::CommandExt;

        let mut shell = Command::new("cmd");
        shell.creation_flags(CREATE_NO_WINDOW);
        let [disable_autorun, execute, command] = windows_shell_args(command);
        // cmd.exe parses shell syntax, not C-runtime argument escaping.
        // Keep the user's inner quotes intact; cmd removes this outer pair.
        shell
            .args([disable_autorun, execute])
            .raw_arg(format!("\"{command}\""));
        shell
    };

    #[cfg(not(windows))]
    let mut shell = {
        use std::os::unix::process::CommandExt;

        let mut shell = Command::new("sh");
        shell.args(["-c", command]);
        // Own process group (setpgid in the child, before exec) so the timeout
        // can kill `sh -c "sleep …"` *and* its sleep grandchild. Killing only
        // the shell leaves the grandchild alive, holding the pipe write end and
        // reparented to init.
        shell.process_group(0);
        shell
    };

    let child = shell
        .current_dir(current_dir)
        // Piped, not inherited: the child must not draw over the TUI, and we
        // still want its output after a kill.
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    #[cfg(windows)]
    remember_job(&child);
    Ok(child)
}

#[cfg(windows)]
fn remember_job(child: &Child) {
    if let Ok(job) = crate::windows::process_tree_job(child) {
        let id = child.id();
        ACTIVE_JOBS.with(|jobs| {
            let mut jobs = jobs.borrow_mut();
            jobs.retain(|(existing_id, _)| *existing_id != id);
            jobs.push((id, job));
        });
    }
}

/// Disarm kill-on-close when a shell exits normally so any backgrounded child
/// survives after its job handle is released.
pub(super) fn detach_shell_tree(child: &Child) {
    #[cfg(windows)]
    {
        let id = child.id();
        ACTIVE_JOBS.with(|jobs| {
            let mut jobs = jobs.borrow_mut();
            for (_, handle) in jobs.iter().filter(|(existing_id, _)| *existing_id == id) {
                drop(crate::windows::disarm_kill_on_close(handle));
            }
            jobs.retain(|(existing_id, _)| *existing_id != id);
        });
    }
    #[cfg(not(windows))]
    let _ = child;
}

/// How long a command has to act on SIGHUP before its group gets SIGKILL.
#[cfg(unix)]
const HANG_UP_GRACE: Duration = Duration::from_secs(1);

/// How often the grace period looks at the shell.
#[cfg(unix)]
const GRACE_POLL: Duration = Duration::from_millis(10);

/// Tear down a timed-out / cancelled shell, including grandchildren.
pub(super) fn kill_shell_tree(child: &mut Child) {
    // Best effort throughout: the timeout or the cancel is what gets reported,
    // and a child that already exited has nothing left to kill or reap.
    #[cfg(windows)]
    {
        let id = child.id();
        ACTIVE_JOBS.with(|jobs| {
            jobs.borrow_mut()
                .retain(|(existing_id, _)| *existing_id != id);
        });
    }
    #[cfg(unix)]
    hang_up_then_kill(child);
    drop(child.kill());
    drop(child.wait());
}

/// Stop the shell's group the way a shell stops its jobs when the terminal
/// goes: SIGHUP, so a program can clean up, then SIGKILL for whatever is left
/// once the shell is gone or the grace period is over.
///
/// The shell leads its own group (set in `spawn_shell`), and signalling a
/// whole group has no std equivalent. Nothing here reaps it, so its id
/// cannot name a stranger's group when the SIGKILL goes out.
#[cfg(unix)]
fn hang_up_then_kill(child: &Child) {
    if crate::platform::hang_up_process_group(child).is_ok() {
        let deadline = Instant::now() + HANG_UP_GRACE;
        // An error means nothing is left to wait for.
        while Instant::now() < deadline && !crate::platform::has_exited(child).unwrap_or(true) {
            std::thread::sleep(GRACE_POLL);
        }
    }
    drop(crate::platform::kill_process_group(child));
}

#[cfg(test)]
mod tests;
