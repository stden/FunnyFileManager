//! Purpose: Wipe (`Alt+Del`) — Far's careful delete: each file is overwritten
//! with zeros, truncated, renamed to a temporary name, and only then removed.
//!
//! Why four steps for one deletion: an unlinked file leaves its bytes on the
//! disk until something else claims the blocks, and its name in the directory
//! index until that slot is reused. The zeros cover the bytes, the truncation
//! hands the covered blocks back, and the temporary name covers the name.
//! Far's `System.WipeSymbol` (another filler byte, or random ones) is not
//! offered: zeros are its documented default and the one value a test can
//! read back.
//!
//! Directories recurse — their files are wiped, then each directory takes a
//! temporary name and goes. Symlinks are unlinked and never followed, the
//! rule F8 and the move fallback already keep: wiping through
//! `logs -> /var/log` would zero the target's files. Pipes, sockets and
//! devices are never opened: they only take the temporary name and go. A
//! file with other hard links goes the same way, and the summary says so:
//! its bytes belong to those names too.
//!
//! Reached from the Files menu row and from `Alt+Del`, both of which resolve
//! to `MenuCommand::Wipe`; `ops::menu` hands that command here.

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use crate::models::{App, Overlay};
use crate::status::{FailedEntry, Status};
use crate::utils::{refresh_panels, unique_destination};

/// One buffer of zeros per write. Larger gains nothing against the page
/// cache; smaller spends a syscall on every few KiB of a big file.
const ZERO_CHUNK: usize = 64 * 1024;

/// The name an entry carries in its last moment. A dotfile, so a listing
/// refreshed mid-wipe does not show it as something the user made.
const TEMP_PREFIX: &str = ".ffm-wipe";

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

/// The moments a wipe can be observed at, for the test seam.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    /// The zeros are on disk and the file still has its length.
    Overwritten,
    /// The entry has its temporary name and is about to be removed.
    Renamed,
}

/// What a test installs to watch a wipe happen.
#[cfg(any(test, tarpaulin))]
pub type Observer = Box<dyn FnMut(Stage, &Path)>;

#[cfg(any(test, tarpaulin))]
thread_local! {
    /// Test seam: called at each `Stage` with the path as it is at that moment.
    /// The zeros are the point of the feature and the next step erases them,
    /// so this is the only place they can be read back.
    pub static WITNESS: std::cell::RefCell<Option<Observer>> =
        const { std::cell::RefCell::new(None) };
}

#[cfg(any(test, tarpaulin))]
fn witness(stage: Stage, path: &Path) {
    WITNESS.with(|witness| {
        if let Some(observe) = witness.borrow_mut().as_mut() {
            observe(stage, path);
        }
    });
}

#[cfg(not(any(test, tarpaulin)))]
const fn witness(_stage: Stage, _path: &Path) {}

/// `Alt+Del`: ask before wiping, the way F8 asks before deleting.
pub fn open(app: &mut App) {
    let paths = crate::ops::get_source_paths(app);
    if paths.is_empty() {
        // The cursor is on `..`: there is no dialog about nothing.
        app.set_status(Status::NothingToWipe);
        return;
    }
    app.set_overlay(Overlay::WipeConfirm(paths));
}

/// The dialog's Cancel: nothing was touched, and the bar says so.
pub fn cancel(app: &mut App) {
    app.close_overlay();
    app.set_status(Status::WipeCancelled);
}

/// The dialog's Wipe: every confirmed entry, with per-entry failures folded
/// into the summary the way F8 folds its own.
pub fn run(app: &mut App) {
    let Overlay::WipeConfirm(paths) = &app.overlay else {
        return;
    };
    let paths = paths.clone();
    app.close_overlay();

    let mut count = 0;
    let mut shared = 0;
    let mut errors = Vec::new();
    for path in &paths {
        match wipe_path(path) {
            Ok(kept) => {
                count += 1;
                shared += kept;
            }
            Err(error) => errors.push(FailedEntry::new(path, &error)),
        }
    }

    app.get_active_panel().clear_marks();
    app.set_status(summary(count, shared, errors));
    // The entries are gone whatever the reload says; reporting a refresh
    // failure here would replace "Wiped" with an error about intact data
    // (the same reasoning as `ops::file::refresh_after_operation`).
    drop(refresh_panels(app));
}

/// An error outranks the hard-link note: it is the news the user must act on.
fn summary(count: usize, shared: usize, errors: Vec<FailedEntry>) -> Status {
    let error_count = errors.len();
    match errors.into_iter().next() {
        None if shared > 0 => Status::WipedSharedFiles { count, shared },
        None => Status::Wiped { count },
        Some(first_error) => Status::WipedWithErrors {
            count,
            error_count,
            first_error,
        },
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
        fs::remove_file(path)?;
        return Ok(0);
    }
    if metadata.is_dir() {
        let shared = wipe_children(path)?;
        rename_then_remove(path, Removal::Dir)?;
        return Ok(shared);
    }
    let shared = metadata.is_file() && crate::directory::link_count(&metadata) > 1;
    if metadata.is_file() && !shared {
        zero_fill(path)?;
    }
    rename_then_remove(path, Removal::File)?;
    Ok(usize::from(shared))
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
    let mut file = OpenOptions::new().write(true).open(path)?;
    let mut remaining = file.metadata()?.len();
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
    removal.apply(&temporary)
}

/// A name nothing in the entry's own directory has: the rename must stay on
/// one filesystem, and the suffix rule is the one every copy already uses.
fn temporary_name(path: &Path) -> io::Result<PathBuf> {
    let dir = path
        .parent()
        .ok_or_else(|| io::Error::other("no parent directory"))?;
    unique_destination(dir, std::ffi::OsStr::new(TEMP_PREFIX))
}

#[cfg(test)]
mod tests;
