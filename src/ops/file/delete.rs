//! Purpose: F8: confirm, then remove every selected entry without following
//! links, streaming progress, supporting Esc / Ctrl+C cancellation, and
//! reporting the first entry that refused.

use crate::command::language::SettingsPage;
use std::io;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use super::removal::remove_path_with_cancel;
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
    if !app.settings.enabled(SettingsPage::Confirmations) {
        return handle_delete(app);
    }
    Ok(())
}

/// Perform the confirmed delete operation, polling Esc / Ctrl+C once shown.
pub fn handle_delete(app: &mut App) -> io::Result<()> {
    handle_delete_with_cancel(app, super::super::live_cancel())
}

/// Perform the confirmed delete operation, stopping cleanly if `cancel` asks to.
pub fn handle_delete_with_cancel(
    app: &mut App,
    mut cancel: impl FnMut(bool) -> bool,
) -> io::Result<()> {
    let Some(files) = app.take_delete_confirm() else {
        return Ok(());
    };
    let removal = Instant::now();
    let outcome = delete_all(&files, &mut cancel);
    app.performance
        .record(REMOVE_ACTION, removal.elapsed(), Duration::ZERO);
    app.get_active_panel().clear_marks();
    match outcome {
        DeleteOutcome::Completed(count) => app.set_status(Status::Deleted { count }),
        DeleteOutcome::Cancelled => app.set_status(Status::DeleteCancelled),
        DeleteOutcome::Stopped(stopped_at) => stopped(app, &files, &stopped_at),
    }
    let refresh = Instant::now();
    refresh_after_operation(app);
    app.performance
        .record(REFRESH_ACTION, refresh.elapsed(), Duration::ZERO);
    Ok(())
}

/// Where F8 stopped on an OS refusal, and why.
struct StoppedAt {
    done: usize,
    left: usize,
    failed: FailedEntry,
    denial: Option<crate::models::Denial>,
}

enum DeleteOutcome {
    Completed(usize),
    Cancelled,
    Stopped(StoppedAt),
}

/// Say where F8 stopped, and why.
fn stopped(app: &mut App, files: &[PathBuf], outcome: &StoppedAt) {
    app.set_status(Status::DeletedWithErrors {
        count: outcome.done,
        error_count: 1,
        first_error: outcome.failed.clone(),
    });
    let base = app.get_active_panel_ref().path.clone();
    let stop_info = super::Stop {
        operation: crate::progress::Operation::Delete,
        failed: &outcome.failed,
        denial: outcome.denial.clone(),
        done: outcome.done,
        remaining: super::remaining(files, outcome.left),
        base: &base,
        selected: files.len(),
        settings: None,
    };
    super::stop(app, &stop_info);
}

/// Remove `files` one by one without following links, stopping at cancellation
/// or at the first entry that refuses.
fn delete_all(files: &[PathBuf], cancel: &mut dyn FnMut(bool) -> bool) -> DeleteOutcome {
    let _run = crate::progress::begin_delete(files);
    for (index, target) in files.iter().enumerate() {
        if let Err(error) = remove_path_with_cancel(target, cancel) {
            if error.kind() == io::ErrorKind::Interrupted {
                return DeleteOutcome::Cancelled;
            }
            let at = FailedAt::of(&error).map_or(target.as_path(), FailedAt::path);
            let denial = super::denied::classify(at, &error);
            return DeleteOutcome::Stopped(StoppedAt {
                done: index,
                left: files.len() - index - 1,
                failed: FailedEntry::new(at, &error),
                denial,
            });
        }
    }
    DeleteOutcome::Completed(files.len())
}
