//! Purpose: The tool rows, which remove the files or folders one `TempScope`
//! names under its folder. A scope is measured and removed only where it
//! matches: a symbolic link is neither followed nor matched, a folder is
//! entered only where the scope says so, and a folder is never removed as one
//! of a file pattern's matches.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::{remove_path, size_of, CleanupOutcome};
use crate::models::TempScope;

impl CleanupOutcome {
    /// Remove every match of `scope` under `root`. A missing `root` has none.
    pub(super) fn remove_scope(&mut self, root: &Path, scope: TempScope) {
        match matches(root, scope) {
            Ok(paths) => {
                for path in paths {
                    _ = self.record(remove_match(&path, scope));
                }
            }
            Err(error) => _ = self.record(Err(error)),
        }
    }
}

/// Remove one match: a folder whole, a file only as a file. The kind is fixed
/// by the scope, so a folder that has taken a marker's name since the listing
/// is refused rather than removed.
fn remove_match(path: &Path, scope: TempScope) -> io::Result<u64> {
    if scope.names_folders() {
        return remove_path(path);
    }
    let size = size_of(path)?;
    fs::remove_file(path).map(|()| size)
}

/// The bytes the matches of `scope` under `root` hold; `None` when there are none.
pub(super) fn measure(root: &Path, scope: TempScope) -> io::Result<Option<u64>> {
    let paths = matches(root, scope)?;
    if paths.is_empty() {
        return Ok(None);
    }
    paths
        .iter()
        .map(|path| size_of(path))
        .sum::<io::Result<u64>>()
        .map(Some)
}

/// What `scope` matches under `root`: regular files, or for
/// `GradleTransforms` the folders.
fn matches(root: &Path, scope: TempScope) -> io::Result<Vec<PathBuf>> {
    match scope {
        TempScope::MavenPartialDownloads => maven_markers(root),
        TempScope::GradleDaemonLogs => gradle_daemon_logs(root),
        TempScope::GradleTransforms => folders_named(root, |name| name.starts_with("transforms-")),
    }
}

/// The partial-download markers in `dir` and in every folder below it. A
/// folder that cannot be read is skipped: the rest of the repository still counts.
fn maven_markers(dir: &Path) -> io::Result<Vec<PathBuf>> {
    let mut found = files_named(dir, |name| {
        name.ends_with(".lastUpdated") || name == "resolver-status.properties"
    })?;
    for folder in folders_named(dir, |_| true)? {
        found.extend(maven_markers(&folder).unwrap_or_default());
    }
    Ok(found)
}

/// The `*.log` files directly in each Gradle version folder (`8.11.1`) under
/// `daemon`. Only folders named as a version are entered, so the registry and
/// the running daemon's state beside the logs are left alone.
fn gradle_daemon_logs(daemon: &Path) -> io::Result<Vec<PathBuf>> {
    let versions = folders_named(daemon, |name| {
        name.starts_with(|c: char| c.is_ascii_digit())
    })?;
    let mut found = Vec::new();
    for version in versions {
        found.extend(files_named(&version, is_log).unwrap_or_default());
    }
    Ok(found)
}

fn is_log(name: &str) -> bool {
    Path::new(name)
        .extension()
        .is_some_and(|extension| extension == "log")
}

fn files_named(dir: &Path, keep: impl Fn(&str) -> bool) -> io::Result<Vec<PathBuf>> {
    named_entries(dir, keep, is_regular_file)
}

fn folders_named(dir: &Path, keep: impl Fn(&str) -> bool) -> io::Result<Vec<PathBuf>> {
    named_entries(dir, keep, |kind| kind.is_dir())
}

/// The entries directly in `dir` whose kind `is_kind` accepts and whose name
/// `keep` accepts. The kind is the one the listing reports, so a link is never
/// taken for the file or folder it points at. A missing `dir` has no entries.
fn named_entries(
    dir: &Path,
    keep: impl Fn(&str) -> bool,
    is_kind: impl Fn(fs::FileType) -> bool,
) -> io::Result<Vec<PathBuf>> {
    let listing = match fs::read_dir(dir) {
        Ok(listing) => listing,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };
    let mut paths = Vec::new();
    for entry in listing {
        let entry = entry?;
        let name = entry.file_name();
        if is_kind(entry.file_type()?) && keep(&name.to_string_lossy()) {
            paths.push(entry.path());
        }
    }
    Ok(paths)
}

#[expect(
    clippy::filetype_is_file,
    reason = "a regular file only: a pipe, device or socket with the same name is not one"
)]
fn is_regular_file(kind: fs::FileType) -> bool {
    kind.is_file()
}

#[cfg(test)]
mod tests;
