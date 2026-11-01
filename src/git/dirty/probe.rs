//! Bounded, sanitized `git status` subprocess probe for a single repository.

use std::fs::File;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

/// Upper bound on a single `git status` probe so a slow mount or wedged
/// process cannot freeze directory navigation.
const STATUS_TIMEOUT: Duration = Duration::from_secs(5);

/// Poll interval while waiting for `git status` to emit a byte or exit.
const POLL_INTERVAL: Duration = Duration::from_millis(5);

/// Environment variables that would redirect `git -C <repo>` to a different
/// repository or index when inherited from the parent environment.
const SANITIZED_GIT_ENV: [&str; 5] = [
    "GIT_DIR",
    "GIT_WORK_TREE",
    "GIT_COMMON_DIR",
    "GIT_INDEX_FILE",
    "GIT_OBJECT_DIRECTORY",
];

/// State files inside `.git` that mark an unfinished operation.
pub(super) const IN_PROGRESS: [&str; 4] = [
    "MERGE_HEAD",
    "REBASE_HEAD",
    "CHERRY_PICK_HEAD",
    "REVERT_HEAD",
];

pub(super) fn query_dirty(directory: &Path, git_dir: &Path) -> bool {
    if IN_PROGRESS.iter().any(|name| git_dir.join(name).exists()) {
        return true;
    }
    let Ok(stdout) = tempfile::tempfile() else {
        return false;
    };
    let Ok(stdout_writer) = stdout.try_clone() else {
        return false;
    };
    let mut command = status_command(directory, stdout_writer);
    let Ok(mut child) = command.spawn() else {
        return false;
    };
    wait_for_status(&mut child, &stdout, STATUS_TIMEOUT)
}

pub(super) fn status_command(directory: &Path, stdout: File) -> Command {
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
        .args(["status", "--porcelain", "-unormal", "--no-renames"])
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GCM_INTERACTIVE", "Never")
        .stdin(Stdio::null())
        .stdout(stdout)
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

pub(super) fn wait_for_status(child: &mut Child, stdout: &File, timeout: Duration) -> bool {
    #[cfg(windows)]
    let job = crate::windows::process_tree_job(child).ok();
    let deadline = Instant::now() + timeout;
    loop {
        if has_output(stdout) {
            stop_child(child);
            #[cfg(windows)]
            drop(job);
            return true;
        }
        match child.try_wait() {
            Ok(Some(_)) => return has_output(stdout),
            Ok(None) if Instant::now() < deadline => {
                std::thread::sleep(POLL_INTERVAL);
            }
            Ok(None) | Err(_) => {
                stop_child(child);
                #[cfg(windows)]
                drop(job);
                return has_output(stdout);
            }
        }
    }
}

fn has_output(stdout: &File) -> bool {
    stdout.metadata().is_ok_and(|meta| meta.len() > 0)
}

fn stop_child(child: &mut Child) {
    #[cfg(unix)]
    drop(crate::platform::kill_process_group(child));
    drop(child.kill());
    drop(child.wait());
}
