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
mod replace;
mod transfer;

pub use create::{handle_create_file, handle_mkdir};
pub use delete::{handle_delete, handle_delete_confirm};
pub use dialog::copy_with;
pub use transfer::{handle_copy, handle_move};

// `merge` and `replace` reach these as `super::…`.
use entry::copy_entry;
use placement::{finish, place, place_over_existing, remove_existing};
// The unit tests below reach these through `use super::*`.
#[cfg(test)]
use {
    crate::copy::Purpose,
    crate::models::Overlay,
    crate::status::{Landed, Status},
    crate::utils::CopyRules,
    placement::is_newer,
    transfer::{copy_into, move_into},
};
// Only the Unix symlink tests reach this one.
#[cfg(all(test, unix))]
use placement::reject_transfer_onto_itself;

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

/// Mutex to serialize tests that depend on the global `FORCE_RENAME_FAILURE` flag.
#[cfg(test)]
pub static RENAME_TEST_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Try to rename (move) a file or directory; returns an error on failure.
/// In test mode, can be forced to fail to simulate cross-device moves.
pub fn rename_or_fail(source: &Path, dest: &Path) -> io::Result<()> {
    #[cfg(any(test, tarpaulin))]
    let forced = FORCE_RENAME_FAILURE.load(Ordering::SeqCst);

    #[cfg(not(any(test, tarpaulin)))]
    let forced = false;

    if forced {
        Err(io::Error::other("forced rename failure"))
    } else {
        fs::rename(source, dest)
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
mod tests_mkdir_errors;
#[cfg(test)]
#[cfg(unix)]
mod tests_modes;
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
#[cfg(unix)]
mod tests_sparse;
#[cfg(test)]
#[cfg(unix)]
mod tests_special_files;
#[cfg(test)]
mod tests_symlinks;
#[cfg(test)]
mod tests_transfer_policies;
