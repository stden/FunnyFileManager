//! Purpose: F8: confirm, then remove every selected entry without following
//! links, reporting how many went and the first that would not.

use std::io;

use super::entry::remove_path;
use super::{get_source_paths, refresh_after_operation};
use crate::models::{App, Overlay};
use crate::status::{FailedEntry, Status};

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

    let mut deleted_count = 0;
    let mut errors = Vec::new();

    for target in &files {
        // Same rule as the move fallback: never resolve a link before removing
        // it, or F8 on `logs -> /var/log` would attack the target.
        match remove_path(target) {
            Ok(()) => deleted_count += 1,
            Err(error) => errors.push(FailedEntry::new(target, &error)),
        }
    }

    // Clear marks after deletion
    app.get_active_panel().clear_marks();

    if errors.is_empty() {
        app.set_status(Status::Deleted {
            count: deleted_count,
        });
    } else {
        app.set_status(Status::DeletedWithErrors {
            count: deleted_count,
            error_count: errors.len(),
            first_error: errors.first().cloned().unwrap_or_default(),
        });
    }

    refresh_after_operation(app);
    Ok(())
}
