//! Purpose: Names of the executables reachable through `$PATH`.
//!
//! The first word of a shell command is a program name, and until now the
//! completer only knew the handful of built-in verbs — so `car<Tab>` could not
//! reach `cargo`. This is the source that fixes that.

use std::collections::BTreeSet;
use std::ffi::OsString;
use std::path::Path;
use std::sync::{Once, OnceLock};

/// The memoized scan of the process `$PATH`, shared by [`executables`],
/// [`ready`] and [`warm`].
static EXECUTABLES: OnceLock<Vec<String>> = OnceLock::new();

/// Every executable name on `$PATH`, sorted and de-duplicated.
///
/// Memoized for the life of the process: `$PATH` does not change inside a
/// running FFM, and re-reading ~40 directories on every `Tab` would be felt on
/// a cold cache. The scan itself is [`scan`], which takes the variable as an
/// argument so tests can point it at a `TempDir` without touching the
/// process-global environment.
pub fn executables() -> &'static [String] {
    EXECUTABLES.get_or_init(|| scan(std::env::var_os("PATH").as_deref()))
}

/// The scan, once it has finished; `None` while it has not. Never blocks, so
/// a dialog can offer `PATH` programs without waiting for them.
pub fn ready() -> Option<&'static [String]> {
    EXECUTABLES.get().map(Vec::as_slice)
}

/// Start the scan on a background thread, once per process.
///
/// The scan calls `metadata` for every entry of every `PATH` directory, which
/// takes seconds on WSL's `/mnt/c/…` entries: the UI thread must not wait for
/// it. A thread that cannot be started is logged, and [`ready`] stays `None`
/// until something scans on its own thread.
pub fn warm() {
    static STARTED: Once = Once::new();
    STARTED.call_once(|| {
        let spawned = std::thread::Builder::new()
            .name("ffm-path-scan".to_owned())
            .spawn(|| {
                executables();
            });
        if let Err(error) = spawned {
            crate::diagnostics::log_error(&format!("PATH scan: {error}"));
        }
    });
}

/// Executable names found in the `PATH`-shaped variable `path_var`.
///
/// A directory that cannot be read contributes nothing and does not stop the
/// scan — the same rule the rest of completion follows, because a broken entry
/// on `$PATH` must not cost the user their `Tab` key. The `BTreeSet` both
/// de-duplicates (the same name usually appears in several directories) and
/// gives a deterministic order for the ranker to work from.
#[must_use]
pub fn scan(path_var: Option<&std::ffi::OsStr>) -> Vec<String> {
    let Some(path_var) = path_var else {
        return Vec::new();
    };
    let mut names = BTreeSet::new();
    for dir in std::env::split_paths(path_var) {
        collect_executables(&dir, &mut names);
    }
    names.into_iter().collect()
}

/// Add every executable file directly inside `dir` to `names`.
fn collect_executables(dir: &Path, names: &mut BTreeSet<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        if !is_executable(&entry) {
            continue;
        }
        // Lossy is right here: a name we cannot render is a name the user
        // cannot type either, so an approximation beats dropping it.
        names.insert(entry.file_name().to_string_lossy().into_owned());
    }
}

/// Whether a directory entry is something the shell could run.
///
/// Unix asks the permission bits; Windows has no execute bit, so any file is a
/// candidate and the extension (`.exe`, `.cmd`, …) is left in the name — that
/// is what the user types there anyway.
///
/// The metadata is fetched through [`std::fs::metadata`], which **follows
/// symlinks**, and not through `DirEntry::metadata`, which does not. Most of
/// `/usr/bin` is symlinks, and a symlink's own mode is `0o777`: asking the link
/// would pass every one of them, including links pointing at directories, and
/// would call a dangling link runnable.
fn is_executable(entry: &std::fs::DirEntry) -> bool {
    #[cfg(not(unix))]
    if let Ok(kind) = entry.file_type() {
        if !kind.is_symlink() {
            return !kind.is_dir();
        }
    }
    let Ok(metadata) = std::fs::metadata(entry.path()) else {
        return false;
    };
    if metadata.is_dir() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}

/// Executable names starting with `prefix`, or nothing for an empty prefix.
///
/// **Prefix, not fuzzy**, unlike every other source here. `$PATH` holds
/// thousands of names, and a subsequence match on two characters returns most
/// of them — a dropdown that long is not a choice, it is a wall. Bash completes
/// commands by prefix for the same reason.
///
/// An empty prefix returns nothing rather than the whole of `$PATH`: `Tab` at
/// an empty prompt should offer the built-in verbs, not three thousand
/// programs.
#[must_use]
pub fn matching(prefix: &str) -> Vec<String> {
    matching_in(executables(), prefix)
}

/// The names in `names` starting with `prefix`, or nothing for an empty
/// prefix: [`matching`]'s one prefix rule, over sorted names, as returned by
/// [`scan`] and [`executables`]. Only the matching range is visited.
#[must_use]
pub fn matching_in(names: &[String], prefix: &str) -> Vec<String> {
    if prefix.is_empty() {
        return Vec::new();
    }
    let start = names.partition_point(|name| name.as_str() < prefix);
    names
        .get(start..)
        .unwrap_or_default()
        .iter()
        .take_while(|name| name.starts_with(prefix))
        .cloned()
        .collect()
}

/// The `PATH`-shaped value joining `dirs`, for tests and callers assembling one.
#[must_use]
pub fn join_paths<'a>(dirs: impl IntoIterator<Item = &'a Path>) -> Option<OsString> {
    std::env::join_paths(dirs).ok()
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_ranges;
