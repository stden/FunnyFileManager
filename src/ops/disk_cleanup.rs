//! Purpose: Disk Cleanup operation — scan candidate sizes, open the dialog,
//! and remove the selected entries when the user confirms.
//!
//! Entry point: `:cleanup`

use std::io;
use std::path::Path;

use crate::models::{App, CleanupField, CleanupKind, DiskCleanupDialog, Overlay};
use crate::status::Status;
use crate::utils::refresh_panels;

mod files;

/// `:cleanup` — measure the default candidates and open the checklist dialog.
pub fn open(app: &mut App) -> io::Result<()> {
    open_dialog(app, DiskCleanupDialog::new());
    Ok(())
}

fn open_dialog(app: &mut App, mut dialog: DiskCleanupDialog) {
    // Resolve relative paths (the Rust `target` dir) against the active panel.
    let panel_path = app.get_active_panel_ref().path.clone();
    measure(&mut dialog, &panel_path);
    app.set_overlay(Overlay::DiskCleanup(Box::new(dialog)));
}

/// Cancel button / Esc: close without touching anything.
pub fn cancel(app: &mut App) {
    app.close_overlay();
    app.set_status(Status::CommandOutput("Cleanup cancelled".to_owned()));
}

/// "Clean up" button: delete every checked, existing candidate, then report.
pub fn run(app: &mut App) {
    let Overlay::DiskCleanup(ref dialog) = app.overlay else {
        return;
    };
    // Own the selection so the overlay can close before filesystem effects begin.
    let selected: Vec<_> = dialog
        .candidates
        .iter()
        .filter(|candidate| candidate.checked && candidate.exists)
        .cloned()
        .collect();
    app.close_overlay();

    if selected.is_empty() {
        app.set_status(Status::CommandOutput(
            "Nothing selected for cleanup".to_owned(),
        ));
        return;
    }

    let mut message = cleanup_summary(&files::cleanup(&selected));
    if let Err(error) = refresh_panels(app) {
        crate::diagnostics::log_error(&format!("Cleanup panel refresh failed: {error}"));
        message.push_str("; panels could not be refreshed");
    }
    app.set_status(Status::CommandOutput(message));
}

fn cleanup_summary(outcome: &files::CleanupOutcome) -> String {
    let size = outcome.freed;
    let errors = outcome.failures;
    if errors == 0 {
        format!("Freed {size}")
    } else {
        format!("Freed {size}, {errors} item(s) could not be removed")
    }
}

/// Fill in `exists` and `size` for every candidate in `dialog`.
///
/// Relative paths are resolved against `base` (the active panel directory).
/// The JVM heap-dump candidate scans for `*.hprof` files instead of sizing
/// the whole home directory.
pub fn measure(dialog: &mut DiskCleanupDialog, base: &Path) {
    for candidate in &mut dialog.candidates {
        if !candidate.path.is_absolute() {
            candidate.path = base.join(&candidate.path);
        }
        candidate.size = files::measure(candidate).unwrap_or_else(|error| {
            crate::diagnostics::log_error(&format!("Cleanup scan failed: {error}"));
            None
        });
        candidate.exists = candidate.size.is_some();
        candidate.checked = candidate.exists
            && (candidate.kind == CleanupKind::HeapDumps || candidate.label.contains("Temp"));
    }
}

/// Move the cursor forward (down / Tab) or backward (up / Shift+Tab).
pub fn advance_field(dialog: &mut DiskCleanupDialog, forward: bool) {
    let n = dialog.candidates.len();
    dialog.field = match (dialog.field, forward) {
        (CleanupField::Item(i), true) if i + 1 < n => CleanupField::Item(i + 1),
        (CleanupField::Item(_), true) | (CleanupField::Cancel, false) => CleanupField::Run,
        (CleanupField::Run, true) | (CleanupField::Item(0), false) => CleanupField::Cancel,
        (CleanupField::Cancel, true) => CleanupField::Item(0),
        (CleanupField::Run, false) => CleanupField::Item(n.saturating_sub(1)),
        (CleanupField::Item(i), false) => CleanupField::Item(i - 1),
    };
}

/// Toggle the checkbox for the currently focused item (if it exists on disk).
pub fn toggle_checked(dialog: &mut DiskCleanupDialog) {
    if let CleanupField::Item(idx) = dialog.field {
        if let Some(c) = dialog.candidates.get_mut(idx) {
            if c.exists {
                c.checked = !c.checked;
            }
        }
    }
}

// ── convenience wrappers used by popup handler ────────────────────────────────

/// Toggle the focused item then advance to the next field.
pub fn toggle_and_advance(app: &mut App) {
    let Overlay::DiskCleanup(ref mut dialog) = app.overlay else {
        return;
    };
    toggle_checked(dialog);
    advance_field(dialog, true);
}

/// Advance without toggling (arrow keys / Tab on a button).
pub fn move_focus(app: &mut App, forward: bool) {
    let Overlay::DiskCleanup(ref mut dialog) = app.overlay else {
        return;
    };
    advance_field(dialog, forward);
}

/// Space on a checkbox row.
pub fn space_checked(app: &mut App) {
    let Overlay::DiskCleanup(ref mut dialog) = app.overlay else {
        return;
    };
    toggle_checked(dialog);
}

/// Returns the current focused field without borrowing `app` for the whole
/// match expression in the popup handler.
pub fn current_field(app: &App) -> Option<CleanupField> {
    let Overlay::DiskCleanup(ref dialog) = app.overlay else {
        return None;
    };
    Some(dialog.field)
}

#[cfg(test)]
mod tests;
