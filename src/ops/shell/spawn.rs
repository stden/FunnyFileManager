//! Purpose: Start the platform shell with piped output in a process group of
//! its own, and tear the whole group down when it has to stop.

use std::process::{Child, Command, Stdio};
use std::{io, path::Path};

#[cfg(windows)]
use super::windows_shell_args;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

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

    shell
        .current_dir(current_dir)
        // Piped, not inherited: the child must not draw over the TUI, and we
        // still want its output after a kill.
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
}

/// Tear down a timed-out / cancelled shell, including Unix grandchildren.
pub(super) fn kill_shell_tree(child: &mut Child) {
    // The shell leads its own group (set in `spawn_shell`), and killing a
    // whole group has no std equivalent. It is not reaped yet, so its id
    // cannot name a stranger's group.
    // Best effort throughout: the timeout or the cancel is what gets reported,
    // and a child that already exited has nothing left to kill or reap.
    #[cfg(unix)]
    drop(crate::platform::kill_process_group(child));
    drop(child.kill());
    drop(child.wait());
}
