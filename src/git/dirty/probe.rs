//! Bounded, sanitized `git status` subprocess probe for a single repository.

use std::fs::File;
use std::path::Path;
use std::process::{Child, Command};
use std::time::{Duration, Instant};

use crate::git::command::{git_command, stop_child};

/// Upper bound on a single `git status` probe so a slow mount or wedged
/// process cannot freeze directory navigation.
const STATUS_TIMEOUT: Duration = Duration::from_secs(5);

/// Initial and maximum poll intervals while waiting for `git status`.
const POLL_START: Duration = Duration::from_micros(250);
const POLL_MAX: Duration = Duration::from_millis(5);

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
    run_status_probe(directory).unwrap_or(false)
}

fn run_status_probe(directory: &Path) -> Option<bool> {
    let stdout = tempfile::tempfile().ok()?;
    let stdout_writer = stdout.try_clone().ok()?;
    let mut child = status_command(directory, stdout_writer).spawn().ok()?;
    Some(wait_for_status(&mut child, &stdout, STATUS_TIMEOUT))
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
    let mut poll = POLL_START;
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
                std::thread::sleep(poll);
                poll = (poll * 2).min(POLL_MAX);
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
