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

use crate::models::{App, Overlay};
use crate::status::{FailedEntry, Status};
use crate::utils::refresh_panels;

mod steps;
mod witness;

pub use steps::wipe_path;
pub use witness::Stage;

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

#[cfg(test)]
mod tests;
