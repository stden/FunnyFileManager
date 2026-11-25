//! Purpose: Remove one entry on disk without following it: a link is unlinked,
//! never walked, and a folder tree is removed in parallel while naming the
//! first inner entry that refused.

use std::{fs, io, path::Path};

mod parallel;

/// Remove `path` without ever following it.
///
/// `symlink_metadata` keeps the decision on the link itself, so a link to a
/// directory is unlinked and never walked. The elevated helper calls it on
/// Windows; other platforms compile it only so the tests can.
#[cfg(any(windows, test))]
pub(super) fn remove_path(path: &Path) -> io::Result<()> {
    remove_path_with_cancel(path, &mut |_| false)
}

/// Remove `path` without following links, stopping with `Interrupted` if
/// `cancel` asks to after progress has been shown.
pub(super) fn remove_path_with_cancel(
    path: &Path,
    cancel: &mut dyn FnMut(bool) -> bool,
) -> io::Result<()> {
    let file_type = fs::symlink_metadata(path)?.file_type();
    if !file_type.is_dir() {
        remove_link_or_file(path, file_type)?;
        let painted = crate::progress::delete_progress(1, 1, Some(path));
        if cancel(painted) {
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "delete cancelled",
            ));
        }
        return Ok(());
    }
    parallel::remove_tree(path, cancel)
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

#[cfg(any(test, tarpaulin))]
thread_local! {
    pub(super) static FORCE_REMOVAL_FAILURE: std::cell::Cell<Option<RemovalStep>> =
        const { std::cell::Cell::new(None) };
}

/// Refuse `step` with `PermissionDenied` when a test forced it.
#[cfg(any(test, tarpaulin))]
pub(super) fn forced_with(forced_step: Option<RemovalStep>, step: RemovalStep) -> io::Result<()> {
    if forced_step == Some(step) {
        return Err(io::Error::from(io::ErrorKind::PermissionDenied));
    }
    Ok(())
}

/// Remove a file or a link on the calling thread.
pub(super) fn remove_link_or_file(path: &Path, file_type: fs::FileType) -> io::Result<()> {
    remove_link_or_file_with(
        path,
        file_type,
        #[cfg(any(test, tarpaulin))]
        FORCE_REMOVAL_FAILURE.get(),
    )
}

/// Remove a file or a link. A link to a folder is itself a folder entry on
/// Windows, removed with `remove_dir`, which never touches its target; a
/// read-only file there is removed as `remove_dir_all` removes it.
pub(super) fn remove_link_or_file_with(
    path: &Path,
    file_type: fs::FileType,
    #[cfg(any(test, tarpaulin))] forced_step: Option<RemovalStep>,
) -> io::Result<()> {
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
    forced_with(forced_step, RemovalStep::RemoveFile)?;
    match fs::remove_file(path) {
        #[cfg(windows)]
        Err(error) if file_type.is_symlink() => fs::remove_dir(path).or(Err(error)),
        Err(error) if error.kind() == io::ErrorKind::PermissionDenied => {
            remove_read_only(path, false).or(Err(error))
        }
        removed => removed,
    }
}

/// Remove an empty folder, clearing a Windows read-only attribute if needed.
pub(super) fn remove_empty_dir(
    folder: &Path,
    #[cfg(any(test, tarpaulin))] forced_step: Option<RemovalStep>,
) -> io::Result<()> {
    match try_remove_dir(
        folder,
        #[cfg(any(test, tarpaulin))]
        forced_step,
    ) {
        Err(error) if error.kind() == io::ErrorKind::PermissionDenied => {
            remove_read_only(folder, true).or(Err(error))
        }
        removed => removed,
    }
}

fn try_remove_dir(
    folder: &Path,
    #[cfg(any(test, tarpaulin))] forced_step: Option<RemovalStep>,
) -> io::Result<()> {
    #[cfg(any(test, tarpaulin))]
    forced_with(forced_step, RemovalStep::RemoveDir)?;
    fs::remove_dir(folder)
}

/// Windows refuses to remove a read-only file or folder; clear the attribute
/// and restore it if the removal still fails.
#[cfg(windows)]
#[expect(
    clippy::permissions_set_readonly_false,
    reason = "Windows: clears the read-only attribute of a file or folder about to be removed"
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
        drop(fs::set_permissions(path, permissions));
    })
}

#[cfg(not(windows))]
fn remove_read_only(_path: &Path, _is_dir: bool) -> io::Result<()> {
    Err(io::Error::from(io::ErrorKind::PermissionDenied))
}

#[cfg(all(test, windows))]
mod tests;
