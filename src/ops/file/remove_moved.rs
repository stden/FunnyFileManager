//! Purpose: finish a move across filesystems — remove from the source only
//! what the copy holds, and say what stayed behind.

use std::{
    fmt, fs, io,
    path::{Path, PathBuf},
};

use crate::utils::name_is_taken;

/// A move across filesystems whose copy landed whole, but part of whose
/// source stayed where it was: `path` could not be removed, for `reason`.
///
/// Carried inside an `io::Error`, like `recording::TerminalLost`, so it
/// passes through every `?` between the removal and the batch that reports
/// it, which counts the entry as moved *and* names what stayed. "Moved 0
/// item(s), 1 error(s)" was the old report: a user who believed it deleted
/// the "extra" copy, which by then held the only copy of what the removal had
/// already taken.
#[derive(Debug)]
pub(super) struct LeftBehind {
    /// Where the whole copy is.
    pub(super) moved_to: PathBuf,
    /// The first thing the source kept.
    pub(super) path: PathBuf,
    reason: io::Error,
}

impl fmt::Display for LeftBehind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "the copy is complete, but this stayed in the source: {}",
            self.reason
        )
    }
}

impl std::error::Error for LeftBehind {}

impl LeftBehind {
    /// The `LeftBehind` `error` carries, if it carries one.
    pub(super) fn of(error: &io::Error) -> Option<&Self> {
        error.get_ref()?.downcast_ref()
    }
}

/// Remove `source` now that `copy` holds all of it — only what the copy
/// holds, deepest first, and never anything else.
///
/// An entry the copy lacks appeared after it was made, and stays; so does
/// anything that will not go. The walk carries on past both, because
/// everything else is safely in the copy, and reports the first as a
/// [`LeftBehind`]. Links are removed, never followed.
pub(super) fn remove_moved(source: &Path, copy: &Path) -> io::Result<()> {
    let mut first = None;
    remove_entry(source, copy, &mut first);
    match first {
        None => Ok(()),
        Some((path, reason)) => Err(io::Error::new(
            reason.kind(),
            LeftBehind {
                moved_to: copy.to_path_buf(),
                path,
                reason,
            },
        )),
    }
}

type Kept = Option<(PathBuf, io::Error)>;

fn remove_entry(source: &Path, copy: &Path, first: &mut Kept) {
    if !name_is_taken(copy) {
        let reason = io::Error::other("it appeared after the copy was made");
        return keep(first, source, reason);
    }
    let removed = match fs::symlink_metadata(source).map(|meta| meta.file_type()) {
        Ok(kind) if kind.is_dir() => {
            remove_contents(source, copy, first);
            remove_dir(source)
        }
        Ok(kind) => remove_file(source, kind),
        Err(error) => Err(error),
    };
    match removed {
        Ok(()) => crate::progress::removed(source),
        Err(reason) => keep(first, source, reason),
    }
}

fn remove_dir(source: &Path) -> io::Result<()> {
    #[cfg(any(test, tarpaulin))]
    forced_refusal(source)?;
    fs::remove_dir(source)
}

fn remove_file(source: &Path, kind: fs::FileType) -> io::Result<()> {
    #[cfg(any(test, tarpaulin))]
    forced_refusal(source)?;
    super::entry::remove_link_or_file(source, kind)
}

pub(super) fn remove_contents(folder: &Path, copy: &Path, first: &mut Kept) {
    remove_entries(
        folder,
        copy,
        first,
        fs::read_dir(folder)
            .map(|entries| entries.map(|entry| entry.map(|e| (e.path(), e.file_name())))),
    );
}

pub(super) fn remove_entries(
    folder: &Path,
    copy: &Path,
    first: &mut Kept,
    entries: io::Result<impl Iterator<Item = io::Result<(PathBuf, std::ffi::OsString)>>>,
) {
    let entries = match entries {
        Ok(entries) => entries,
        Err(reason) => return keep(first, folder, reason),
    };
    for entry in entries {
        match entry {
            Ok((path, name)) => remove_entry(&path, &copy.join(name), first),
            Err(reason) => keep(first, folder, reason),
        }
    }
}

/// Remember the first thing that stayed; a folder that stayed only because
/// of it is not news.
fn keep(first: &mut Kept, path: &Path, reason: io::Error) {
    first.get_or_insert_with(|| (path.to_path_buf(), reason));
}

// Refuses to remove an entry of this name, as the kernel refuses a user an
// entry of a folder they may not write to — which root never is.
// Thread-local: the move runs on the calling thread.
#[cfg(any(test, tarpaulin))]
thread_local! {
    pub static FORCE_REMOVAL_REFUSAL: std::cell::Cell<Option<&'static str>> =
        const { std::cell::Cell::new(None) };
}

#[cfg(any(test, tarpaulin))]
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
