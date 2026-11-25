//! Purpose: How FFM runs `git` in a repository it only reads: the same flags,
//! environment and process group for every probe, so none of them takes a lock,
//! prompts for a password or follows an inherited `GIT_DIR`.

use std::path::Path;
use std::process::{Child, Command, Stdio};

/// Environment variables that would redirect `git -C <repo>` to a different
/// repository or index when inherited from the parent environment.
const SANITIZED_GIT_ENV: [&str; 5] = [
    "GIT_DIR",
    "GIT_WORK_TREE",
    "GIT_COMMON_DIR",
    "GIT_INDEX_FILE",
    "GIT_OBJECT_DIRECTORY",
];

/// `git <arguments>` in `directory`, configured the way every probe runs it: no
/// optional locks, no fsmonitor, no prompts, no submodule recursion, stdin and
/// stderr closed. The caller chooses stdout, and stdin when it has input.
pub(super) fn git_command(directory: &Path, arguments: &[&str]) -> Command {
    let mut command = Command::new("git");
    command
        .args([
            "--no-optional-locks",
            "-c",
            "core.fsmonitor=false",
            "-c",
            "credential.interactive=false",
            "-c",
            "submodule.recurse=false",
            "-C",
        ])
        .arg(directory)
        .args(arguments)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GCM_INTERACTIVE", "Never")
        .stdin(Stdio::null())
        .stderr(Stdio::null());
    for key in SANITIZED_GIT_ENV {
        command.env_remove(key);
    }
    configure_child_group(&mut command);
    command
}

fn configure_child_group(command: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
}

/// Kills a git probe that ran past its time limit, together with the process
/// group it started.
pub(super) fn stop_child(child: &mut Child) {
    #[cfg(unix)]
    drop(crate::platform::kill_process_group(child));
    drop(child.kill());
    drop(child.wait());
}
