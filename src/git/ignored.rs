//! Purpose: Which entries of a listed directory git ignores, so the panel can
//! dim them, and the `.git` entry of a repository.

use std::fs::File;
use std::io::{Seek, SeekFrom, Write};
use std::path::Path;
use std::process::{Child, Stdio};
use std::time::{Duration, Instant};

use super::command::{git_command, stop_child};
use super::find_repo;
use crate::models::{FileName, PanelMetadata};
use crate::values::entry::DirEntry;

mod cache;
pub(super) use cache::invalidate_under;

/// Upper bound on one `git check-ignore`, as for the status probe, so a slow
/// mount cannot freeze the listing.
const CHECK_TIMEOUT: Duration = Duration::from_secs(5);

/// Initial and maximum poll intervals while `git check-ignore` runs.
const POLL_START: Duration = Duration::from_micros(250);
const POLL_MAX: Duration = Duration::from_millis(5);

/// The repository's own directory. No rule ignores it, but it is not part of the
/// work tree either, so it is dimmed with the ignored entries.
const GIT_DIR_NAME: &str = ".git";

/// Marks, in `metadata`, the entries of `entries` that git ignores in
/// `directory`, and `.git` when it is listed. Nothing is marked outside a
/// repository, and only `.git` when git does not answer in time.
pub(super) fn load_ignored(directory: &Path, entries: &[DirEntry], metadata: &mut PanelMetadata) {
    let Some((repo_root, git_dir)) = find_repo(directory) else {
        return;
    };
    let mut names = Vec::with_capacity(entries.len());
    for entry in entries {
        if entry.name == GIT_DIR_NAME {
            metadata
                .ignored
                .insert(FileName::new(GIT_DIR_NAME.to_owned()));
        } else {
            names.push(entry.name.as_str());
        }
    }
    for name in ignored_names(directory, repo_root, &git_dir, &names) {
        metadata.ignored.insert(FileName::new(name));
    }
}

/// The names among `names` that `git check-ignore` reports in `directory`, or
/// none when the probe fails or runs past [`CHECK_TIMEOUT`].
fn ignored_names(
    directory: &Path,
    repo_root: &Path,
    git_dir: &Path,
    names: &[&str],
) -> Vec<String> {
    if names.is_empty() {
        return Vec::new();
    }
    let stamp = cache::IgnoreStamp::capture(directory, repo_root, git_dir, names);
    if let Some(cached) = cache::lookup(directory, stamp) {
        return cached;
    }
    let Some(found) = run_check_ignore(directory, names) else {
        return Vec::new();
    };
    cache::store(directory, stamp, &found);
    found
}

#[cfg(test)]
thread_local! {
    pub(super) static CHECK_IGNORE_RUNS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

fn run_check_ignore(directory: &Path, names: &[&str]) -> Option<Vec<String>> {
    #[cfg(test)]
    CHECK_IGNORE_RUNS.with(|runs| runs.set(runs.get() + 1));
    let mut input = tempfile::tempfile().ok()?;
    write_nul_separated(&mut input, names).ok()?;
    input.seek(SeekFrom::Start(0)).ok()?;
    let output = tempfile::tempfile().ok()?;
    let mut reader = output.try_clone().ok()?;
    let mut command = git_command(directory, &["check-ignore", "-z", "--stdin"]);
    command
        .stdin(Stdio::from(input))
        .stdout(Stdio::from(output));
    let mut child = command.spawn().ok()?;
    wait_for_exit(&mut child, CHECK_TIMEOUT)?;
    reader.seek(SeekFrom::Start(0)).ok()?;
    let text = std::io::read_to_string(reader).ok()?;
    Some(
        text.split('\0')
            .filter(|name| !name.is_empty())
            .map(str::to_owned)
            .collect(),
    )
}

/// `names` written as NUL-terminated records in one buffered write.
fn write_nul_separated(file: &mut File, names: &[&str]) -> std::io::Result<()> {
    let total: usize = names.iter().map(|name| name.len() + 1).sum();
    let mut buf = Vec::with_capacity(total);
    for name in names {
        buf.extend_from_slice(name.as_bytes());
        buf.push(0);
    }
    file.write_all(&buf)
}

/// Waits for `child` to exit. A probe that runs past `timeout` is stopped, and
/// `None` says there is no answer to read.
pub(super) fn wait_for_exit(child: &mut Child, timeout: Duration) -> Option<()> {
    let deadline = Instant::now() + timeout;
    let mut poll = POLL_START;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return Some(()),
            Ok(None) if Instant::now() < deadline => {
                std::thread::sleep(poll);
                poll = (poll * 2).min(POLL_MAX);
            }
            Ok(None) | Err(_) => {
                stop_child(child);
                return None;
            }
        }
    }
}

#[cfg(test)]
mod tests;
#[cfg(all(test, unix))]
mod tests_deadline;
