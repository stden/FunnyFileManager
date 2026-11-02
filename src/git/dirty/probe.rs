//! Bounded, sanitized `git status` subprocess probe for a single repository.

use std::fs::File;
use std::path::Path;
use std::process::{Child, Command};
use std::time::{Duration, Instant};

use crate::git::command::{git_command, stop_child};

/// Upper bound on a single `git status` probe so a slow mount or wedged
/// process cannot freeze directory navigation.
const STATUS_TIMEOUT: Duration = Duration::from_secs(5);

/// Poll interval while waiting for `git status` to emit a byte or exit.
const POLL_INTERVAL: Duration = Duration::from_millis(5);

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
    let mut command = git_command(
        directory,
        &["status", "--porcelain", "-unormal", "--no-renames"],
    );
    command.stdout(stdout);
    command
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
