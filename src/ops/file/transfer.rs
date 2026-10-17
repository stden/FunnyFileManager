//! Purpose: F5/F6 straight to the other panel: copy or move every selected
//! entry under a free name, and report the batch as one outcome.

use std::{
    io,
    path::{Path, PathBuf},
};

use super::entry::{copy_entry, reject_transfer_into_itself};
use super::{get_source_paths, refresh_after_operation, remove_moved, rename_within, Renamed};
use crate::copy::Purpose;
use crate::models::App;
use crate::status::{FailedEntry, Landed, Status};
use crate::utils::{unique_destination, CopyRules};

mod outcome;
pub(in crate::ops::file) use outcome::BatchOutcome;

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
        crate::progress::copying_across(source);
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

fn transfer_all(
    sources: &[PathBuf],
    target_dir: &Path,
    transfer: fn(&Path, &Path) -> io::Result<PathBuf>,
) -> BatchOutcome {
    let mut outcome = BatchOutcome::default();
    for (index, source) in sources.iter().enumerate() {
        outcome.record(source, transfer(source, target_dir));
        crate::progress::source_done();
        if outcome.stopped() {
            outcome.left = sources.len() - index - 1;
            break;
        }
    }
    outcome
}

pub fn handle_copy(app: &mut App) -> io::Result<()> {
    quick_transfer(app, Purpose::Copy);
    Ok(())
}

pub fn handle_move(app: &mut App) -> io::Result<()> {
    quick_transfer(app, Purpose::Move);
    Ok(())
}

/// F5 or F6 straight to the other panel, stopping at the first entry that
/// fails.
fn quick_transfer(app: &mut App, purpose: Purpose) {
    let source_paths = get_source_paths(app);
    if source_paths.is_empty() {
        return;
    }

    let target_dir = app.get_inactive_panel().path.clone();
    let operation = crate::progress::Operation::of(purpose);
    let run = crate::progress::begin(operation, &source_paths);
    let transfer = match purpose {
        Purpose::Copy => copy_into,
        Purpose::Move => move_into,
    };
    let outcome = transfer_all(&source_paths, &target_dir, transfer);
    drop(run);

    app.get_active_panel().clear_marks();
    report_quick(app, purpose, &source_paths, &target_dir, outcome);
    refresh_after_operation(app);
}

/// Say what the quick F5/F6 did, and where it stopped if it did.
fn report_quick(
    app: &mut App,
    purpose: Purpose,
    sources: &[PathBuf],
    target_dir: &Path,
    outcome: BatchOutcome,
) {
    let Some(first_error) = outcome.errors.first() else {
        app.set_status(Status::Transferred {
            purpose,
            landed: Landed::of(outcome.count, outcome.first_dest),
            skipped: 0,
            renamed: 0,
        });
        return;
    };
    let status = stopped_status(purpose, outcome.count, outcome.errors.len(), first_error);
    app.set_status(status);
    let stopped = super::Stop {
        operation: crate::progress::Operation::of(purpose),
        failed: first_error,
        denial: outcome.denial.clone(),
        done: outcome.count,
        remaining: super::remaining(sources, outcome.left),
        base: target_dir,
        selected: sources.len(),
        settings: None,
    };
    super::stop(app, &stopped);
}

/// "Copied N item(s), M error(s): …" or its move twin.
pub(super) fn stopped_status(
    purpose: Purpose,
    count: usize,
    error_count: usize,
    first_error: &FailedEntry,
) -> Status {
    let first_error = first_error.clone();
    match purpose {
        Purpose::Copy => Status::CopiedWithErrors {
            count,
            error_count,
            first_error,
        },
        Purpose::Move => Status::MovedWithErrors {
            count,
            error_count,
            first_error,
        },
    }
}
