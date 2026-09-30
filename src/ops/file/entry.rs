//! Purpose: One entry on disk: copy it under an exclusively claimed name,
//! remove it without following links, and refuse a directory sent into itself.

use std::{fs, io, path::Path};

#[cfg(any(test, tarpaulin))]
use super::FORCE_TRANSFER_COLLISION;
use crate::utils::{copy_dir_all_with, copy_file, copy_link, CopyRules};

/// Refuse to copy or move a directory into itself or into one of its children.
///
/// The destination comes from the *other* panel, which may well sit inside the
/// directory under the cursor; the recursive copy would then descend into the
/// copy it is writing and duplicate the tree until the path length gives out.
/// Far Manager and mc both refuse this outright.
pub(super) fn reject_transfer_into_itself(source: &Path, target_dir: &Path) -> io::Result<()> {
    // Canonical paths, so `..` and symlinked panel paths cannot slip past. The
    // target may not exist yet — a typed `project/archive` is created on the
    // way — so its closest existing ancestor stands in for it; otherwise a
    // nested destination inside the source would slip past this check and be
    // created before the copy itself refused.
    let (Ok(real_source), Some(real_target)) = (
        source.canonicalize(),
        crate::utils::canonical_existing_ancestor(target_dir),
    ) else {
        return Ok(()); // unreadable paths fail with a real error further down
    };
    if real_target.starts_with(&real_source) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "cannot copy a directory into itself",
        ));
    }
    Ok(())
}

/// Copy `source` onto `dest`, removing a half-written `dest` if it fails — a
/// partial copy under the final name is indistinguishable from a complete one.
///
/// Directories are claimed with exclusive `create_dir` first; files with
/// `create_new`. Only paths we successfully claimed are cleaned up on failure
/// (an `AlreadyExists` means another process owns the name — leave it alone).
/// Callers retry on [`io::ErrorKind::AlreadyExists`].
pub(super) fn copy_entry(source: &Path, dest: &Path, rules: CopyRules) -> io::Result<()> {
    #[cfg(any(test, tarpaulin))]
    if FORCE_TRANSFER_COLLISION.get() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "forced destination collision",
        ));
    }
    // The link itself decides, before anything that would follow it: both
    // `Path::is_dir` and `fs::copy` resolve links, so `logs -> /var/log` under
    // the cursor would be transferred as a full copy of the target tree — and
    // the move fallback would then delete the user's link. `copy_dir_all`
    // already recreates nested links; the top-level entry must match.
    if fs::symlink_metadata(source).is_ok_and(|metadata| metadata.is_symlink())
        && !crate::utils::follows_link(rules.links, source)
    {
        return copy_link(source, dest, rules);
    }

    if source.is_dir() {
        return copy_directory_entry(source, dest, rules);
    }

    // Refuses a FIFO, socket or device before opening it or claiming `dest`.
    copy_file(source, dest, rules)
}

fn copy_directory_entry(source: &Path, dest: &Path, rules: CopyRules) -> io::Result<()> {
    // Exclusive claim of the directory name, then fill it. `copy_dir_all_with`
    // is happy if `dest` already exists (it `create_dir_all`s).
    fs::create_dir(dest).map_err(|error| crate::utils::FailedAt::wrap(dest, error))?;
    let result = copy_dir_all_with(source, dest, rules);
    if result.is_err() {
        // Best effort: the copy's own error is the one worth reporting.
        drop(fs::remove_dir_all(dest));
    }
    result
}

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
    use crate::utils::FailedAt;

    crate::progress::entering(folder);
    for entry in fs::read_dir(folder).map_err(|error| FailedAt::wrap(folder, error))? {
        let entry = entry.map_err(|error| FailedAt::wrap(folder, error))?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|error| FailedAt::wrap(&path, error))?;
        if file_type.is_dir() {
            remove_tree_reporting(&path)?;
        } else {
            remove_link_or_file(&path, file_type).map_err(|error| FailedAt::wrap(&path, error))?;
            crate::progress::removed(&path);
        }
    }
    fs::remove_dir(folder).map_err(|error| FailedAt::wrap(folder, error))
}

/// Remove a file or a link. A link to a folder is itself a folder entry on
/// Windows, removed with `remove_dir`, which never touches its target; a
/// read-only file there is removed as `remove_dir_all` removes it.
fn remove_link_or_file(path: &Path, file_type: fs::FileType) -> io::Result<()> {
    match fs::remove_file(path) {
        Err(error) if cfg!(windows) && file_type.is_symlink() => {
            fs::remove_dir(path).or(Err(error))
        }
        Err(error) if error.kind() == io::ErrorKind::PermissionDenied => {
            remove_read_only(path).or(Err(error))
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
    reason = "Windows: clears the read-only attribute of a file about to be removed"
)]
fn remove_read_only(path: &Path) -> io::Result<()> {
    let permissions = fs::symlink_metadata(path)?.permissions();
    if !permissions.readonly() {
        return Err(io::Error::from(io::ErrorKind::PermissionDenied));
    }
    let mut writable = permissions.clone();
    writable.set_readonly(false);
    fs::set_permissions(path, writable)?;
    fs::remove_file(path).inspect_err(|_| {
        // Best effort: the removal's error is the one worth reporting.
        drop(fs::set_permissions(path, permissions));
    })
}

/// A Unix file's own mode does not decide whether it can be removed: its
/// folder's does.
#[cfg(not(windows))]
fn remove_read_only(_path: &Path) -> io::Result<()> {
    Err(io::Error::from(io::ErrorKind::PermissionDenied))
}
