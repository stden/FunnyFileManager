//! Purpose: F5/F6 straight to the other panel: copy or move every selected
//! entry under a free name, and report the batch as one outcome.

use std::{
    io,
    path::{Path, PathBuf},
};

use super::entry::{copy_entry, reject_transfer_into_itself};
use super::remove_moved::LeftBehind;
use super::{get_source_paths, refresh_after_operation, remove_moved, rename_within, Renamed};
use crate::copy::Purpose;
use crate::models::App;
use crate::status::{FailedEntry, Landed, Status};
use crate::utils::FailedAt;
use crate::utils::{unique_destination, CopyRules};

/// How many times to re-pick a destination when exclusive create races another
/// process that claimed the same free name between `unique_destination` and
/// the open/create.
const MAX_TRANSFER_ATTEMPTS: u32 = 10_000;

pub(super) fn copy_into(source: &Path, target_dir: &Path) -> io::Result<PathBuf> {
    reject_transfer_into_itself(source, target_dir)?;
    let file_name = source.file_name().unwrap_or_default();
    for _ in 0..MAX_TRANSFER_ATTEMPTS {
        let dest = unique_destination(target_dir, file_name)?;
        match copy_entry(source, &dest, CopyRules::PLAIN) {
            Ok(()) => return Ok(dest),
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(e),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        format!(
            "could not find exclusive destination for {}",
            file_name.to_string_lossy()
        ),
    ))
}

pub(super) fn move_into(source: &Path, target_dir: &Path) -> io::Result<PathBuf> {
    reject_transfer_into_itself(source, target_dir)?;
    let file_name = source.file_name().unwrap_or_default();
    for _ in 0..MAX_TRANSFER_ATTEMPTS {
        let dest = unique_destination(target_dir, file_name)?;
        if rename_within(source, &dest)? == Renamed::Moved {
            return Ok(dest);
        }

        // Across filesystems: exclusive copy + delete. `?` after a successful
        // exclusive copy so the source is only removed once the whole copy
        // has landed.
        match copy_entry(source, &dest, CopyRules::MOVE) {
            Ok(()) => {
                remove_moved(source, &dest)?;
                return Ok(dest);
            }
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(e),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        format!(
            "could not find exclusive destination for {}",
            file_name.to_string_lossy()
        ),
    ))
}

/// What a batch of copies/moves achieved. One failing item must not abandon the
/// rest of the batch, so failures are collected instead of propagated.
#[derive(Default)]
pub(super) struct BatchOutcome {
    pub(super) count: usize,
    pub(super) first_dest: Option<PathBuf>,
    pub(super) errors: Vec<FailedEntry>,
}

impl BatchOutcome {
    pub(super) fn record(&mut self, source: &Path, result: io::Result<PathBuf>) {
        let error = match result {
            Ok(dest) => return self.landed(dest),
            Err(error) => error,
        };
        // Which file, and why — kept apart until something decides how to
        // show them. Formatting here threw the two away in one string.
        if let Some(left) = LeftBehind::of(&error) {
            // Moved whole, and part of the source stayed: both are true, and
            // the user acts on each — keep the copy, deal with what stayed.
            self.landed(left.moved_to.clone());
            self.errors.push(FailedEntry::new(&left.path, left));
            return;
        }
        let failed = FailedAt::of(&error).map_or(source, |at| at.path());
        self.errors.push(FailedEntry::new(failed, &error));
    }

    fn landed(&mut self, dest: PathBuf) {
        self.first_dest.get_or_insert(dest);
        self.count += 1;
    }
}

fn transfer_all(
    sources: &[PathBuf],
    target_dir: &Path,
    transfer: fn(&Path, &Path) -> io::Result<PathBuf>,
) -> BatchOutcome {
    let mut outcome = BatchOutcome::default();
    for source in sources {
        outcome.record(source, transfer(source, target_dir));
    }
    outcome
}

pub fn handle_copy(app: &mut App) -> io::Result<()> {
    let source_paths = get_source_paths(app);
    if source_paths.is_empty() {
        return Ok(());
    }

    let target_dir = app.get_inactive_panel().path.clone();
    let outcome = transfer_all(&source_paths, &target_dir, copy_into);

    app.get_active_panel().clear_marks();
    if let Some(first_error) = outcome.errors.first() {
        app.set_status(Status::CopiedWithErrors {
            count: outcome.count,
            error_count: outcome.errors.len(),
            first_error: first_error.clone(),
        });
    } else {
        app.set_status(Status::Transferred {
            purpose: Purpose::Copy,
            landed: Landed::of(outcome.count, outcome.first_dest),
            skipped: 0,
            renamed: 0,
        });
    }
    refresh_after_operation(app);
    Ok(())
}

pub fn handle_move(app: &mut App) -> io::Result<()> {
    let source_paths = get_source_paths(app);
    if source_paths.is_empty() {
        return Ok(());
    }

    let target_dir = app.get_inactive_panel().path.clone();
    let outcome = transfer_all(&source_paths, &target_dir, move_into);

    app.get_active_panel().clear_marks();
    if let Some(first_error) = outcome.errors.first() {
        app.set_status(Status::MovedWithErrors {
            count: outcome.count,
            error_count: outcome.errors.len(),
            first_error: first_error.clone(),
        });
    } else {
        app.set_status(Status::Transferred {
            purpose: Purpose::Move,
            landed: Landed::of(outcome.count, outcome.first_dest),
            skipped: 0,
            renamed: 0,
        });
    }
    refresh_after_operation(app);
    Ok(())
}
