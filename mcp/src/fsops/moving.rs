//! Purpose: Move one entry — by rename within a filesystem, by copy and
//! removal across two — telling the one rename that may become a copy apart
//! from a rename that was refused.

use super::remove_moved::{remove_moved, LeftBehind};
use super::{copy_entry, Purpose};
use std::fs;
use std::io;
use std::path::Path;

// Makes every rename fail with this kind: `CrossesDevices` is what the
// kernel answers between two filesystems, `PermissionDenied` what it answers
// a user who may not write to the source's folder — which root never is.
// Thread-local: a move runs on the calling thread, and parallel tests must
// not see it.
#[cfg(test)]
thread_local! {
    pub static FORCE_RENAME_FAILURE: std::cell::Cell<Option<io::ErrorKind>> =
        const { std::cell::Cell::new(None) };
}

/// What a move achieved.
#[derive(Debug)]
pub enum Moved {
    /// The entry is at its destination and gone from its source.
    Whole,
    /// The copy at the destination is whole, but part of the source stayed.
    Leaving(LeftBehind),
}

/// What renaming an entry to its destination settled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Renamed {
    /// The entry is under its new name; the move is done.
    Moved,
    /// The two names are on different filesystems. Only a copy followed by
    /// a removal can move the entry, which is what `mv` does then too.
    AcrossFilesystems,
}

/// Move `source` to `dest`, which must not exist yet.
///
/// A rename within one filesystem is the whole move, and it is atomic.
/// Across filesystems the kernel answers `EXDEV`, and the only thing left is
/// to copy and then remove. A rename refused for any other reason — no
/// permission on the source's folder, a busy mount point, a read-only
/// filesystem — is the move's own failure: copying anyway used to leave a
/// full copy at the destination beside a source that could not be removed,
/// or merge a folder into the folder already at that name and then remove
/// the source.
///
/// The destination is checked first because a rename replaces what is there
/// — a file, an empty folder, a link — and a tool driven by an assistant must
/// never destroy what it did not move. The check is a moment before the
/// rename; `std` exposes no rename that refuses to replace.
pub fn move_entry(source: &Path, dest: &Path) -> io::Result<Moved> {
    if fs::symlink_metadata(dest).is_ok() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("destination exists: {}", dest.display()),
        ));
    }
    if rename_within(source, dest)? == Renamed::Moved {
        return Ok(Moved::Whole);
    }
    // `?` after the copy: the source is only touched once the whole copy
    // has landed, and a copy that failed has removed what it claimed.
    copy_entry(source, dest, Purpose::Move)?;
    Ok(remove_moved(source, dest))
}

fn rename_within(source: &Path, dest: &Path) -> io::Result<Renamed> {
    match rename(source, dest) {
        Ok(()) => Ok(Renamed::Moved),
        Err(error) if error.kind() == io::ErrorKind::CrossesDevices => {
            Ok(Renamed::AcrossFilesystems)
        }
        Err(error) => Err(error),
    }
}

/// `fs::rename`, or the failure a test forced.
fn rename(source: &Path, dest: &Path) -> io::Result<()> {
    #[cfg(test)]
    if let Some(kind) = FORCE_RENAME_FAILURE.get() {
        return Err(io::Error::new(kind, "forced rename failure"));
    }
    fs::rename(source, dest)
}
