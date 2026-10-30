//! Purpose: Replace a single entry only after its new contents and options
//! succeed — atomically where that keeps what the entry is, through the file
//! where only that does.

use std::{fs, io, path::Path};

use crate::copy::{AccessRights, CopyPrompt, Purpose};
use crate::utils::FailedAt;
use crate::utils::FileMode;

mod stage;

/// How an existing file takes the new contents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Replacement {
    /// Prepared beside it under a temporary name, then renamed over it:
    /// atomic, so no failure and no crash can leave it half-written — but
    /// the file is a new inode, owned by whoever renamed it.
    Staged,
    /// Written through it, keeping its inode, owner, mode and other names;
    /// see [`crate::utils::rewrite_file`] for what that costs.
    #[cfg_attr(
        not(unix),
        expect(dead_code, reason = "Windows has no identity a rename could lose")
    )]
    InPlace,
}

/// A same-filesystem move replaces the entry atomically, and a refused
/// rename fails the move. Everything else is replaced as [`replacement`]
/// decides. Only a committed copy may remove the source of a move across
/// filesystems. A failure that belongs to the destination is reported there.
pub(super) fn entry(source: &Path, dest: &Path, settings: &CopyPrompt) -> io::Result<()> {
    if settings.purpose == Purpose::Move && rename_replacing(source, dest)? == super::Renamed::Moved
    {
        return Ok(());
    }
    match replacement(source, dest, settings) {
        Replacement::InPlace => write_through(source, dest, settings)?,
        Replacement::Staged => stage::stage(source, dest, settings)?,
    }
    if settings.purpose == Purpose::Move {
        super::remove_moved(source, dest)?;
    }
    Ok(())
}

fn rename_replacing(source: &Path, dest: &Path) -> io::Result<super::Renamed> {
    match super::rename_within(source, dest) {
        Err(error) if error.kind() == io::ErrorKind::PermissionDenied => {
            if stage::is_windows_dir_symlink(dest) {
                return Ok(super::Renamed::AcrossFilesystems);
            }
            stage::clear_windows_readonly(dest)
                .and_then(|()| super::rename_within(source, dest))
                .or(Err(error))
        }
        other => other,
    }
}

/// Stage the replacement unless renaming over `dest` would change what it
/// is — another name for it, another owner — or its folder takes no new
/// entry, and it can be written. That is a copy of a file over a file only:
/// a move replaces the name, as `mv` and the rename it stands in for do.
#[cfg(unix)]
pub(super) fn replacement(source: &Path, dest: &Path, settings: &CopyPrompt) -> Replacement {
    let file_over_file = settings.purpose == Purpose::Copy
        && copies_as_file(source, settings)
        && fs::symlink_metadata(dest).is_ok_and(|meta| meta.is_file());
    let rename_would_hurt = crate::file_write::replacing_changes_identity(dest)
        || crate::platform::check_write_access(folder_of(dest)).is_err();
    if file_over_file && rename_would_hurt && crate::platform::check_write_access(dest).is_ok() {
        Replacement::InPlace
    } else {
        Replacement::Staged
    }
}

/// Windows keeps no second name or owner a rename could lose, so the atomic
/// replacement is always the one.
#[cfg(not(unix))]
pub(super) const fn replacement(
    _source: &Path,
    _dest: &Path,
    _settings: &CopyPrompt,
) -> Replacement {
    Replacement::Staged
}

/// Whether the copy of `source` is a regular file: a file, or a link to one
/// the dialog follows.
pub(super) fn copies_as_file(source: &Path, settings: &CopyPrompt) -> bool {
    let is_link = fs::symlink_metadata(source).is_ok_and(|meta| meta.is_symlink());
    (!is_link || crate::utils::follows_link(settings.rules().links, source))
        && fs::metadata(source).is_ok_and(|meta| meta.is_file())
}

pub(super) fn folder_of(dest: &Path) -> &Path {
    dest.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
}

/// An overwrite keeps the mode of the file it replaces, as `cp` does, unless
/// "Access rights: Copy" asks for the source's. A move brings its own, and
/// on Windows the read-only attribute comes from the source, as Far has it.
pub(super) const fn keeps_replaced_mode(settings: &CopyPrompt) -> bool {
    cfg!(unix)
        && matches!(settings.purpose, Purpose::Copy)
        && !matches!(settings.rights, AccessRights::Copy)
}

/// [`Replacement::InPlace`]: the source is opened — its own failures are its
/// own — and the rest happens at the destination.
fn write_through(source: &Path, dest: &Path, settings: &CopyPrompt) -> io::Result<()> {
    let input = crate::utils::open_regular_file(source)?;
    let mut rules = settings.rules();
    if keeps_replaced_mode(settings) {
        rules.modes = FileMode::Fresh;
    }
    ensure_room(&input, dest)
        .and_then(|()| crate::utils::rewrite_file(&input, dest, rules))
        .map_err(|error| FailedAt::wrap(dest, error))
}

/// Refuse while the old contents are whole when the new ones cannot fit in
/// what is free plus what emptying the old file frees — the most common way
/// a write through a file fails half-way. An unknown free space is let
/// through: the write reports its own failure.
#[cfg(unix)]
fn ensure_room(input: &fs::File, dest: &Path) -> io::Result<()> {
    use std::os::unix::fs::MetadataExt;

    let needed = input.metadata()?.blocks().saturating_mul(512);
    let freed = fs::metadata(dest)?.blocks().saturating_mul(512);
    match crate::diskfree::available_bytes(dest) {
        Some(free) if free.saturating_add(freed) < needed => Err(io::Error::new(
            io::ErrorKind::StorageFull,
            "not enough free space to write it",
        )),
        _ => Ok(()),
    }
}

#[cfg(not(unix))]
#[expect(
    clippy::unnecessary_wraps,
    reason = "shares the Unix signature, where the check can refuse"
)]
const fn ensure_room(_input: &fs::File, _dest: &Path) -> io::Result<()> {
    Ok(())
}

/// Canonical paths cannot identify separate hard links. A Unix rename
/// between two names of one inode succeeds without moving either name.
#[cfg(unix)]
pub(super) fn are_hard_links(source: &Path, dest: &Path) -> bool {
    use std::os::unix::fs::MetadataExt;

    let (Ok(source), Ok(dest)) = (fs::symlink_metadata(source), fs::symlink_metadata(dest)) else {
        return false; // The transfer itself reports an unreadable entry.
    };
    source.dev() == dest.dev() && source.ino() == dest.ino()
}

#[cfg(windows)]
pub(super) fn are_hard_links(source: &Path, dest: &Path) -> bool {
    crate::ops::panel_details::windows::are_same_hard_link(source, dest)
}

#[cfg(not(any(unix, windows)))]
pub(super) const fn are_hard_links(_source: &Path, _dest: &Path) -> bool {
    false
}

#[cfg(test)]
mod tests;
