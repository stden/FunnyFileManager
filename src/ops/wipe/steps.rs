//! Purpose: Wipe one entry on disk — zeros over a regular file's bytes, the
//! truncation, a temporary name, then the removal — recursing into folders
//! and never following a link.

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use super::witness::{witness, Stage};
use crate::utils::unique_destination;

/// One buffer of zeros per write. Larger gains nothing against the page
/// cache; smaller spends a syscall on every few KiB of a big file.
pub(super) const ZERO_CHUNK: usize = 64 * 1024;

/// The name an entry carries in its last moment. A dotfile, so a listing
/// refreshed mid-wipe does not show it as something the user made.
pub(super) const TEMP_PREFIX: &str = ".ffm-wipe";

/// Which removal closes a wipe: the two `std::fs` calls differ, and a
/// directory that is not empty by now is a bug worth an error, not a
/// `remove_dir_all` that would quietly skip the zeros.
#[derive(Debug, Clone, Copy)]
enum Removal {
    File,
    Dir,
}

impl Removal {
    fn apply(self, path: &Path) -> io::Result<()> {
        match self {
            Self::File => fs::remove_file(path),
            Self::Dir => fs::remove_dir(path),
        }
    }
}

/// Wipe one entry: a link is unlinked, a directory is emptied and then loses
/// its name like a file does, a file goes through all four steps.
///
/// Only a regular file is opened for its zeros. A pipe, socket or device
/// has no bytes of its own on this disk, and opening one to write blocks
/// until a reader comes (a pipe), fails (a socket) or writes to the device
/// itself — so those lose only their name. So does a file with other hard
/// links: its bytes are theirs too, and zeroing them through this name
/// would empty every other one.
///
/// Returns how many files were only unlinked for that last reason, so the
/// summary can say their data is still on the disk.
pub fn wipe_path(path: &Path) -> io::Result<usize> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.is_symlink() {
        return remove_symlink(path, metadata.file_type()).map(|()| 0);
    }
    if metadata.is_dir() {
        let shared = wipe_children(path)?;
        clear_readonly(path);
        rename_then_remove(path, Removal::Dir)?;
        return Ok(shared);
    }
    #[cfg(unix)]
    let shared = metadata.is_file() && crate::directory::link_count(&metadata) > 1;
    #[cfg(windows)]
    let shared = metadata.is_file() && crate::ops::panel_details::windows::has_multiple_links(path);
    clear_readonly(path);
    if metadata.is_file() && !shared {
        zero_fill(path)?;
    }
    rename_then_remove(path, Removal::File)?;
    Ok(usize::from(shared))
}

fn remove_symlink(path: &Path, file_type: fs::FileType) -> io::Result<()> {
    clear_readonly(path);
    #[cfg(windows)]
    if std::os::windows::fs::FileTypeExt::is_symlink_dir(&file_type) {
        return fs::remove_dir(path).or_else(|_| fs::remove_file(path));
    }
    let _ = file_type;
    fs::remove_file(path)
}

#[expect(
    clippy::permissions_set_readonly_false,
    reason = "clears the read-only attribute before zero-filling and removing a wiped entry"
)]
fn clear_readonly(path: &Path) {
    if let Ok(meta) = fs::symlink_metadata(path) {
        let mut perms = meta.permissions();
        if perms.readonly() {
            perms.set_readonly(false);
            drop(fs::set_permissions(path, perms));
        }
    }
}

fn wipe_children(dir: &Path) -> io::Result<usize> {
    fs::read_dir(dir)?
        .map(|entry| wipe_path(&entry?.path()))
        .sum()
}

/// Steps one and two: zeros over every byte, then no bytes at all. Each
/// pass is synced before the next, or the truncation could reach the disk
/// first and the zeros would cover nothing.
fn zero_fill(path: &Path) -> io::Result<()> {
    let mut options = OpenOptions::new();
    options.write(true);
    let mut file = crate::fileload::open_regular(path, &options)?;
    let meta = file.metadata()?;
    if crate::directory::link_count(&meta) > 1 {
        return Err(io::Error::other(
            "Refusing to wipe a file with multiple hard links",
        ));
    }
    let mut remaining = meta.len();
    let mut zeros = vec![0_u8; ZERO_CHUNK];
    while remaining > 0 {
        // Only the last pass is shorter: as many zeros as bytes are left.
        zeros.truncate(usize::try_from(remaining).unwrap_or(ZERO_CHUNK));
        file.write_all(&zeros)?;
        remaining -= u64::try_from(zeros.len()).map_err(io::Error::other)?;
    }
    file.sync_all()?;
    witness(Stage::Overwritten, path);
    file.set_len(0)?;
    file.sync_all()
}

/// Steps three and four: the entry is removed under a temporary name, so the
/// directory slot its real name occupied is overwritten as well.
fn rename_then_remove(path: &Path, removal: Removal) -> io::Result<()> {
    let temporary = temporary_name(path)?;
    fs::rename(path, &temporary)?;
    witness(Stage::Renamed, &temporary);
    removal.apply(&temporary).inspect_err(|_| {
        drop(fs::rename(&temporary, path));
    })
}

/// A name nothing in the entry's own directory has: the rename must stay on
/// one filesystem, and the suffix rule is the one every copy already uses.
fn temporary_name(path: &Path) -> io::Result<PathBuf> {
    let dir = path
        .parent()
        .ok_or_else(|| io::Error::other("no parent directory"))?;
    unique_destination(dir, std::ffi::OsStr::new(TEMP_PREFIX))
}
