//! Purpose: F8: confirm, then remove every selected entry without following
//! links, reporting how many went and the first that would not.

use std::io;
use std::time::{Duration, Instant};

use super::entry::remove_path;
use super::{get_source_paths, refresh_after_operation};
use crate::models::{App, Overlay};
use crate::status::{FailedEntry, Status};
use crate::utils::FailedAt;

/// The rows an F8 adds under its own keypress row: the removal, then the
/// reload of both panels after it. They name what was done, never a path.
const REMOVE_ACTION: &str = "Delete/Remove";
const REFRESH_ACTION: &str = "Delete/Refresh";

/// Show delete confirmation dialog (F8)
pub fn handle_delete_confirm(app: &mut App) -> io::Result<()> {
    let files_to_delete = get_source_paths(app);
    if files_to_delete.is_empty() {
        return Ok(());
    }

    app.set_overlay(Overlay::DeleteConfirm(files_to_delete));
    if !app
        .settings
        .enabled(crate::menu::SettingsPage::Confirmations)
    {
        return handle_delete(app);
    }
    Ok(())
}

/// Actually perform the delete operation
pub fn handle_delete(app: &mut App) -> io::Result<()> {
    let Some(files) = app.take_delete_confirm() else {
        return Ok(());
    };
    let removal = Instant::now();
    let outcome = delete_all(&files);
    app.performance
        .record(REMOVE_ACTION, removal.elapsed(), Duration::ZERO);
    app.get_active_panel().clear_marks();
    match &outcome.failed {
        None => app.set_status(Status::Deleted {
            count: outcome.done,
        }),
        Some((failed, denial)) => stopped(app, &files, &outcome, failed, denial.clone()),
    }
    let refresh = Instant::now();
    refresh_after_operation(app);
    app.performance
        .record(REFRESH_ACTION, refresh.elapsed(), Duration::ZERO);
    Ok(())
}

/// Say where F8 stopped, and why.
fn stopped(
    app: &mut App,
    files: &[std::path::PathBuf],
    outcome: &Deleted,
    failed: &FailedEntry,
    denial: Option<crate::models::Denial>,
) {
    app.set_status(Status::DeletedWithErrors {
        count: outcome.done,
        error_count: 1,
        first_error: failed.clone(),
    });
    let base = app.get_active_panel_ref().path.clone();
    let stopped = super::Stop {
        operation: crate::progress::Operation::Delete,
        failed,
        denial,
        done: outcome.done,
        remaining: super::remaining(files, outcome.left),
        base: &base,
        selected: files.len(),
        settings: None,
    };
    super::stop(app, &stopped);
}

/// What F8 did: how many went, and where it stopped, and why.
struct Deleted {
    done: usize,
    left: usize,
    failed: Option<(FailedEntry, Option<crate::models::Denial>)>,
}

/// Remove `files` one by one, stopping at the first that will not go, as
/// Far does.
fn delete_all(files: &[std::path::PathBuf]) -> Deleted {
    let _run = crate::progress::begin(crate::progress::Operation::Delete, files);
    for (index, target) in files.iter().enumerate() {
        // Same rule as the move fallback: never resolve a link before removing
        // it, or F8 on `logs -> /var/log` would attack the target.
        if let Err(error) = remove_path(target) {
            // Named by what would not go, however deep in the folder.
            let at = FailedAt::of(&error).map_or(target.as_path(), FailedAt::path);
            let denial = super::denied::classify(at, &error);
            return Deleted {
                done: index,
                left: files.len() - index - 1,
                failed: Some((FailedEntry::new(at, &error), denial)),
            };
        }
    }
    Deleted {
        done: files.len(),
        left: 0,
        failed: None,
    }
}
