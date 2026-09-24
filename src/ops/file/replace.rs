//! Purpose: Replace a single entry only after its new contents and options succeed.

use std::{fs, io, path::Path};

use crate::copy::{CopyPrompt, Purpose};

/// A same-filesystem move replaces the entry atomically. Everything else
/// prepares a temporary sibling before replacing it, so a read, write or
/// metadata failure keeps the destination. Only a committed copy may remove
/// the source of a move across filesystems.
pub(super) fn entry(source: &Path, dest: &Path, settings: &CopyPrompt) -> io::Result<()> {
    if settings.purpose == Purpose::Move && super::rename_or_fail(source, dest).is_ok() {
        return Ok(());
    }
    let folder = dest
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    // copy_entry claims the name exclusively and cleans up a failed copy.
    // Once it succeeds, the temporary owns cleanup for every later failure.
    // A sibling also preserves the meaning of a relative symbolic link.
    let mut staged = tempfile::Builder::new()
        .prefix(".ffm-copy-")
        .make_in(folder, |path| {
            super::copy_entry(source, path, settings.rules())
        })?;
    super::finish(source, staged.path(), settings)?;
    // tempfile::persist clears Windows attributes. This entry was created
    // by copy_entry, so it has no temporary attribute to clear.
    fs::rename(staged.path(), dest)?;
    staged.disable_cleanup(true);
    if settings.purpose == Purpose::Move {
        super::remove_existing(source)?;
    }
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

#[cfg(not(unix))]
pub(super) const fn are_hard_links(_source: &Path, _dest: &Path) -> bool {
    false
}

#[cfg(test)]
mod tests;
