//! Purpose: Which entries of a listed directory git ignores, so the panel can
//! dim them, and the `.git` entry of a repository.

use std::fs::File;
use std::io::{Seek, SeekFrom, Write};
use std::path::Path;
use std::process::{Child, Stdio};
use std::time::{Duration, Instant};

use super::command::{git_command, stop_child};
use super::find_git_dir;
use crate::models::{FileName, PanelMetadata};
use crate::values::entry::DirEntry;

/// Upper bound on one `git check-ignore`, as for the status probe, so a slow
/// mount cannot freeze the listing.
const CHECK_TIMEOUT: Duration = Duration::from_secs(5);

/// Poll interval while `git check-ignore` runs.
const POLL_INTERVAL: Duration = Duration::from_millis(5);

/// The repository's own directory. No rule ignores it, but it is not part of the
/// work tree either, so it is dimmed with the ignored entries.
const GIT_DIR_NAME: &str = ".git";

/// Marks, in `metadata`, the entries of `entries` that git ignores in
/// `directory`, and `.git` when it is listed. Nothing is marked outside a
/// repository, and only `.git` when git does not answer in time.
pub(super) fn load_ignored(directory: &Path, entries: &[DirEntry], metadata: &mut PanelMetadata) {
    if find_git_dir(directory).is_none() {
        return;
    }
    let names: Vec<&str> = entries.iter().map(|entry| entry.name.as_str()).collect();
    if names.contains(&GIT_DIR_NAME) {
        metadata
            .ignored
            .insert(FileName::new(GIT_DIR_NAME.to_owned()));
    }
    for name in ignored_names(directory, &names) {
        metadata.ignored.insert(FileName::new(name));
    }
}

/// The names among `names` that `git check-ignore` reports in `directory`, or
/// none when the probe fails or runs past [`CHECK_TIMEOUT`].
fn ignored_names(directory: &Path, names: &[&str]) -> Vec<String> {
    if names.is_empty() {
        return Vec::new();
    }
    run_check_ignore(directory, names).unwrap_or_default()
}

fn run_check_ignore(directory: &Path, names: &[&str]) -> Option<Vec<String>> {
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

/// `names` written as NUL-terminated records, the form `check-ignore -z` reads.
fn write_nul_separated(file: &mut File, names: &[&str]) -> std::io::Result<()> {
    for name in names {
        file.write_all(name.as_bytes())?;
        file.write_all(&[0])?;
    }
    Ok(())
}

/// Waits for `child` to exit. A probe that runs past `timeout` is stopped, and
/// `None` says there is no answer to read.
fn wait_for_exit(child: &mut Child, timeout: Duration) -> Option<()> {
    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return Some(()),
            Ok(None) if Instant::now() < deadline => std::thread::sleep(POLL_INTERVAL),
            Ok(None) | Err(_) => {
                stop_child(child);
                return None;
            }
        }
    }
}

#[cfg(test)]
mod tests;
