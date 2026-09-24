//! Purpose: One entry on disk: copy it under an exclusively claimed name,
//! remove it without following links, and refuse a directory sent into itself.

use std::{fs, io, path::Path};

#[cfg(any(test, tarpaulin))]
use super::FORCE_TRANSFER_COLLISION;
use crate::utils::{copy_dir_all_with, copy_file, copy_symlink, CopyRules};

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
        return copy_symlink(source, dest);
    }

    if source.is_dir() {
        return copy_directory_entry(source, dest, rules);
    }

    // Refuses a FIFO, socket or device before opening it or claiming `dest`.
    copy_file(source, dest, rules.modes)
}

fn copy_directory_entry(source: &Path, dest: &Path, rules: CopyRules) -> io::Result<()> {
    // Exclusive claim of the directory name, then fill it. `copy_dir_all_with`
    // is happy if `dest` already exists (it `create_dir_all`s).
    fs::create_dir(dest)?;
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
    if fs::symlink_metadata(path)?.is_dir() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
}
