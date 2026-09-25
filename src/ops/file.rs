//! Purpose: Canonical file operations: create, copy, move, mkdir, and delete.
//!
//! This file keeps what every operation shares — the rename seam and the
//! post-operation refresh — and the children own one operation each.

use std::{
    fs, io,
    path::{Path, PathBuf},
};

#[cfg(any(test, tarpaulin))]
use std::sync::atomic::{AtomicBool, Ordering};

use crate::models::App;
use crate::utils::refresh_panels;

mod create;
mod delete;
mod destination;
mod dialog;
mod entry;
mod merge;
mod placement;
mod remove_moved;
mod replace;
mod transfer;

pub use create::{handle_create_file, handle_mkdir};
pub use delete::{handle_delete, handle_delete_confirm};
pub use dialog::copy_with;
pub use transfer::{handle_copy, handle_move};

// `merge` and `replace` reach these as `super::…`.
use entry::copy_entry;
use placement::{place, place_over_existing};
use remove_moved::remove_moved;

#[cfg(any(test, tarpaulin))]
pub static FORCE_RENAME_FAILURE: AtomicBool = AtomicBool::new(false);

// Makes every `copy_entry` report `AlreadyExists`, simulating the race where
// another process claims each destination between the uniqueness check and
// the copy — the only way to drive the transfer loops to exhaustion.
// Thread-local, unlike `FORCE_RENAME_FAILURE`: transfers run on the calling
// thread, and a process-global flag would sabotage every copy test running
// in parallel with the exhaustion tests.
#[cfg(any(test, tarpaulin))]
thread_local! {
    pub static FORCE_TRANSFER_COLLISION: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

// Makes every `rename_or_fail` fail the way a user who may not write to the
// source's folder fails (`EACCES`), which root never does. Thread-local for
// the reason `FORCE_TRANSFER_COLLISION` is.
#[cfg(any(test, tarpaulin))]
thread_local! {
    pub static FORCE_RENAME_DENIAL: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Mutex to serialize tests that depend on the global `FORCE_RENAME_FAILURE` flag.
#[cfg(test)]
pub static RENAME_TEST_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Try to rename (move) a file or directory; returns an error on failure.
/// In test mode, can be forced to fail as a cross-device rename does, or as
/// a refused one does.
pub fn rename_or_fail(source: &Path, dest: &Path) -> io::Result<()> {
    #[cfg(any(test, tarpaulin))]
    if FORCE_RENAME_DENIAL.get() {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "forced rename denial",
        ));
    }
    #[cfg(any(test, tarpaulin))]
    let forced = FORCE_RENAME_FAILURE.load(Ordering::SeqCst);

    #[cfg(not(any(test, tarpaulin)))]
    let forced = false;

    if forced {
        // What the kernel answers for a rename between two filesystems.
        Err(io::Error::new(
            io::ErrorKind::CrossesDevices,
            "forced rename failure",
        ))
    } else {
        fs::rename(source, dest)
    }
}

/// What renaming an entry to its destination settled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Renamed {
    /// The entry is under its new name; the move is done.
    Moved,
    /// The two names are on different filesystems. Only a copy followed by a
    /// removal can move the entry, which is what `mv` does then too.
    AcrossFilesystems,
}

/// Move `source` to `dest` by renaming it, telling a rename across
/// filesystems apart from a rename that was refused.
///
/// Only the first may turn into a copy: `EXDEV` is the kernel saying "not
/// with rename". Any other failure — no permission on the source's folder, a
/// busy mount point, a read-only filesystem — is the move's own failure, and
/// copying anyway used to leave a full copy at the destination beside a
/// source that could not be removed, under a status that said "Moved 0".
fn rename_within(source: &Path, dest: &Path) -> io::Result<Renamed> {
    match rename_or_fail(source, dest) {
        Ok(()) => Ok(Renamed::Moved),
        Err(error) if error.kind() == io::ErrorKind::CrossesDevices => {
            Ok(Renamed::AcrossFilesystems)
        }
        Err(error) => Err(error),
    }
}

/// Reload both panels after an operation that already happened, keeping the
/// operation's own status message.
///
/// WHY the refresh error is dropped — do not turn this back into `?`: by the
/// time it runs, the files are already copied/moved/created/deleted and the
/// status bar already says so. Propagating a *refresh* failure makes the caller
/// replace that with "Delete failed: No such file or directory", telling the
/// user their data is intact when it is gone. F8 on a directory the *other*
/// panel is sitting inside no longer fails that reload — the panel moves up to
/// the nearest folder that exists — but a folder that cannot be read still
/// does. That stale listing is for Ctrl+R to report, on purpose, because there
/// the refresh *is* the operation.
fn refresh_after_operation(app: &mut App) {
    drop(refresh_panels(app));
}

pub fn get_source_paths(app: &App) -> Vec<PathBuf> {
    app.get_active_panel_ref().action_paths()
}

#[cfg(test)]
mod tests_destinations;
#[cfg(test)]
mod tests_exhaustion;
#[cfg(test)]
#[cfg(unix)]
mod tests_kept_metadata;
#[cfg(test)]
mod tests_mkdir_errors;
#[cfg(test)]
#[cfg(unix)]
mod tests_modes;
#[cfg(test)]
#[cfg(unix)]
mod tests_moved_metadata;
#[cfg(test)]
mod tests_moved_sources;
#[cfg(test)]
#[cfg(unix)]
mod tests_non_utf8_names;
#[cfg(test)]
mod tests_overwrite_failures;
#[cfg(test)]
mod tests_overwrite_safety;
#[cfg(test)]
mod tests_refresh_failures;
#[cfg(test)]
mod tests_rename_refusals;
#[cfg(test)]
#[cfg(unix)]
mod tests_sparse;
#[cfg(test)]
#[cfg(unix)]
mod tests_special_files;
#[cfg(test)]
mod tests_symlinks;
#[cfg(test)]
mod tests_transfer_policies;
