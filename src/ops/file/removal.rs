//! Purpose: Remove one entry on disk without following it: a link is unlinked,
//! never walked, and a folder removed under the progress dialog names the
//! entry that refused.

use std::{fs, io, path::Path};

use crate::utils::FailedAt;

/// Remove `path` without ever following it.
///
/// `Path::is_dir` resolves links, so a symlink to a directory would be handed
/// to `fs::remove_dir_all` — which either refuses outright (the link is not a
/// directory) or, on a platform that walks it, takes the *target's* contents
/// with it. `symlink_metadata` keeps the decision on the link itself, so a link
/// is always unlinked and never walked. Used by both the move fallback and F8.
pub(super) fn remove_path(path: &Path) -> io::Result<()> {
    let file_type = fs::symlink_metadata(path)?.file_type();
    if !file_type.is_dir() {
        remove_link_or_file(path, file_type)?;
        crate::progress::removed(path);
        return Ok(());
    }
    if !crate::progress::is_shown() {
        return fs::remove_dir_all(path);
    }
    remove_tree_reporting(path)
}

/// `fs::remove_dir_all`, one entry at a time so that the progress dialog can
/// follow it through a big folder. Never follows a link: `DirEntry::file_type`
/// does not, so a link is unlinked like a file and never walked.
///
/// Stops at the first entry that will not go, naming it (`FailedAt`), so the
/// user is told which file inside the folder refused.
///
/// Walks by path, as the move fallback's removal (`remove_moved`) and the
/// copy walk do. `remove_dir_all` is used when no progress is shown.
fn remove_tree_reporting(folder: &Path) -> io::Result<()> {
    crate::progress::entering(folder);
    #[cfg(any(test, tarpaulin))]
    forced(RemovalStep::ListFolder).map_err(FailedAt::for_path(folder))?;
    for entry in fs::read_dir(folder).map_err(FailedAt::for_path(folder))? {
        #[cfg(any(test, tarpaulin))]
        forced(RemovalStep::NextEntry).map_err(FailedAt::for_path(folder))?;
        let entry = entry.map_err(FailedAt::for_path(folder))?;
        let path = entry.path();
        #[cfg(any(test, tarpaulin))]
        forced(RemovalStep::EntryKind).map_err(FailedAt::for_path(&path))?;
        let file_type = entry.file_type().map_err(FailedAt::for_path(&path))?;
        if file_type.is_dir() {
            remove_tree_reporting(&path)?;
        } else {
            remove_link_or_file(&path, file_type).map_err(FailedAt::for_path(&path))?;
            crate::progress::removed(&path);
        }
    }
    #[cfg(any(test, tarpaulin))]
    forced(RemovalStep::RemoveDir).map_err(FailedAt::for_path(folder))?;
    fs::remove_dir(folder).map_err(FailedAt::for_path(folder))
}

/// The steps of a removal, each of which the kernel refuses in a folder the
/// user may not write to. Root is never refused them, so tests force them.
#[cfg(any(test, tarpaulin))]
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum RemovalStep {
    ListFolder,
    NextEntry,
    EntryKind,
    RemoveFile,
    RemoveDir,
}

// Thread-local: the removal runs on the calling thread.
#[cfg(any(test, tarpaulin))]
thread_local! {
    pub(super) static FORCE_REMOVAL_FAILURE: std::cell::Cell<Option<RemovalStep>> =
        const { std::cell::Cell::new(None) };
}

/// Refuse `step` with `PermissionDenied` when a test forced it.
#[cfg(any(test, tarpaulin))]
fn forced(step: RemovalStep) -> io::Result<()> {
    if FORCE_REMOVAL_FAILURE.get() == Some(step) {
        return Err(io::Error::from(io::ErrorKind::PermissionDenied));
    }
    Ok(())
}

/// Remove a file or a link. A link to a folder is itself a folder entry on
/// Windows, removed with `remove_dir`, which never touches its target; a
/// read-only file there is removed as `remove_dir_all` removes it.
pub(super) fn remove_link_or_file(path: &Path, file_type: fs::FileType) -> io::Result<()> {
    #[cfg(not(windows))]
    let _ = file_type;
    #[cfg(windows)]
    if std::os::windows::fs::FileTypeExt::is_symlink_dir(&file_type) {
        return fs::remove_dir(path).or_else(|error| {
            if error.kind() == io::ErrorKind::PermissionDenied {
                remove_read_only(path, true).or(Err(error))
            } else {
                Err(error)
            }
        });
    }
    #[cfg(any(test, tarpaulin))]
    forced(RemovalStep::RemoveFile)?;
    match fs::remove_file(path) {
        #[cfg(windows)]
        Err(error) if file_type.is_symlink() => fs::remove_dir(path).or(Err(error)),
        Err(error) if error.kind() == io::ErrorKind::PermissionDenied => {
            remove_read_only(path, false).or(Err(error))
        }
        removed => removed,
    }
}

/// Windows refuses to remove a read-only file; `remove_dir_all`, which F8
/// used before the progress dialog, removes it anyway. So does this: the
/// attribute is cleared, and put back if the file still will not go.
#[cfg(windows)]
#[expect(
    clippy::permissions_set_readonly_false,
    reason = "Windows: clears the read-only attribute of a file or link about to be removed"
)]
fn remove_read_only(path: &Path, is_dir: bool) -> io::Result<()> {
    let permissions = fs::symlink_metadata(path)?.permissions();
    if !permissions.readonly() {
        return Err(io::Error::from(io::ErrorKind::PermissionDenied));
    }
    let mut writable = permissions.clone();
    writable.set_readonly(false);
    fs::set_permissions(path, writable)?;
    let removed = if is_dir {
        fs::remove_dir(path)
    } else {
        fs::remove_file(path)
    };
    removed.inspect_err(|_| {
        // Best effort: the removal's error is the one worth reporting.
        drop(fs::set_permissions(path, permissions));
    })
}

/// A Unix file's own mode does not decide whether it can be removed: its
/// folder's does.
#[cfg(not(windows))]
fn remove_read_only(_path: &Path, _is_dir: bool) -> io::Result<()> {
    Err(io::Error::from(io::ErrorKind::PermissionDenied))
}

#[cfg(all(test, windows))]
mod tests;
