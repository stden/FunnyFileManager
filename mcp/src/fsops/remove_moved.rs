//! Purpose: Finish a move across filesystems — remove from the source only
//! what the copy holds, and say what stayed behind.

use super::Moved;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// A move across filesystems whose copy landed whole, but part of whose
/// source stayed where it was: `path` could not be removed, for `reason`.
///
/// Reported beside the copy, never instead of it. The old report was the
/// removal's error alone: a caller who believed the move had failed deleted
/// the "extra" copy, which by then held the only copy of what the removal
/// had already taken.
#[derive(Debug)]
pub struct LeftBehind {
    /// The first thing the source kept.
    pub path: PathBuf,
    pub reason: io::Error,
}

impl fmt::Display for LeftBehind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "the copy is complete, but this stayed in the source: {}: {}",
            self.path.display(),
            self.reason
        )
    }
}

/// Remove `source` now that `copy` holds all of it — only what the copy
/// holds, deepest first, and never anything else.
///
/// An entry the copy lacks appeared after it was made, and stays; so does
/// anything that will not go. The walk carries on past both, because
/// everything else is safely in the copy, and reports the first as
/// [`LeftBehind`]. Links are removed, never followed.
pub fn remove_moved(source: &Path, copy: &Path) -> Moved {
    let mut first = None;
    remove_held(source, copy, &mut first);
    first.map_or(Moved::Whole, Moved::Leaving)
}

type Kept = Option<LeftBehind>;

fn remove_held(source: &Path, copy: &Path, first: &mut Kept) {
    // `symlink_metadata`, not `exists()`: a dangling link in the copy is a
    // copy of a dangling link, and its source goes too.
    if fs::symlink_metadata(copy).is_err() {
        let reason = io::Error::other("it appeared after the copy was made");
        return keep(first, source, reason);
    }
    // A link is never a folder here, so it is never walked into.
    if fs::symlink_metadata(source).is_ok_and(|meta| meta.is_dir()) {
        remove_contents(source, copy, first);
    }
    // A file or a link is unlinked; a folder goes once it is empty — and it
    // is not, if something inside it stayed.
    let removed = forced_refusal(source).and_then(|()| super::remove_entry(source, false));
    if let Err(reason) = removed {
        keep(first, source, reason);
    }
}

fn remove_contents(folder: &Path, copy: &Path, first: &mut Kept) {
    let entries = match fs::read_dir(folder) {
        Ok(entries) => entries,
        Err(reason) => return keep(first, folder, reason),
    };
    for entry in entries {
        match entry {
            Ok(entry) => remove_held(&entry.path(), &copy.join(entry.file_name()), first),
            Err(reason) => keep(first, folder, reason),
        }
    }
}

/// Remember the first thing that stayed; a folder that stayed only because
/// of it is not news.
fn keep(first: &mut Kept, path: &Path, reason: io::Error) {
    first.get_or_insert_with(|| LeftBehind {
        path: path.to_path_buf(),
        reason,
    });
}

// Refuses to remove an entry of this name, as the kernel refuses a user an
// entry of a folder they may not write to — which root never is.
// Thread-local: the move runs on the calling thread.
#[cfg(test)]
thread_local! {
    pub static FORCE_REMOVAL_REFUSAL: std::cell::Cell<Option<&'static str>> =
        const { std::cell::Cell::new(None) };
}

#[cfg(test)]
fn forced_refusal(path: &Path) -> io::Result<()> {
    let refused = FORCE_REMOVAL_REFUSAL.get();
    if refused.is_some_and(|name| path.file_name() == Some(std::ffi::OsStr::new(name))) {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "forced removal refusal",
        ));
    }
    Ok(())
}

#[cfg(not(test))]
#[expect(
    clippy::unnecessary_wraps,
    reason = "outside tests the seam never refuses, but its callers are the same"
)]
const fn forced_refusal(_path: &Path) -> io::Result<()> {
    Ok(())
}
