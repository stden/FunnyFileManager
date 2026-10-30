//! Purpose: `:sessions import netbox [path]` and the F11 import row — where
//! the `.netbox` file is, running the importer, and saying what it did.
//!
//! The importer stays pure over the path it is handed; the default file and
//! the report are decided here. The counts go to the status bar, every
//! skipped entry with its reason to the console transcript.

use std::path::{Path, PathBuf};

use super::{change, report, SessionError};
use crate::models::{App, InputPrompt, InputPurpose, Overlay, Panel, PanelKind};
use crate::sessions::import::{netbox, ImportReport};
use crate::status::Status;

/// The extension a `.netbox` export has.
pub const EXTENSION: &str = "netbox";

/// The F11 row: ask which file to read, offering the `.netbox` file under a
/// file panel's cursor when there is one.
pub fn prompt(app: &mut App) {
    let suggested = file_under_cursor(app)
        .map(|path| path.display().to_string())
        .unwrap_or_default();
    app.set_overlay(Overlay::InputPrompt(Box::new(InputPrompt::new(
        InputPurpose::ImportNetBox,
        &suggested,
    ))));
}

/// Import the `.netbox` file `path` — relative to a file panel, `~`
/// expanded — or, without one, the `.netbox` file under a file panel's
/// cursor.
pub fn run(app: &mut App, path: Option<PathBuf>) -> Result<(), SessionError> {
    let path = match path {
        Some(given) => resolve(app, &given),
        None => file_under_cursor(app).ok_or(SessionError::NoImportSource)?,
    };
    let imported = netbox::import(&path).map_err(SessionError::Import)?;
    let mut outcome = ImportReport::default();
    change(app, |store| {
        outcome = store.adopt(imported);
        Ok(())
    })?;
    announce(app, &outcome);
    Ok(())
}

/// The prompt's answer: import what it names, or report why not.
pub fn submit(app: &mut App, typed: &str) {
    let path = Some(typed.trim())
        .filter(|text| !text.is_empty())
        .map(PathBuf::from);
    let outcome = run(app, path);
    report(app, outcome);
}

/// The counts on the status bar; each skipped entry, and the passwords left
/// out, in the console transcript.
fn announce(app: &mut App, outcome: &ImportReport) {
    for skipped in &outcome.skipped {
        app.console
            .record_output(&format!("NetBox: skipped {skipped}"));
    }
    if outcome.passwords_left_out > 0 {
        app.console.record_output(&format!(
            "NetBox: {} saved password(s) not imported; a password is asked for when connecting",
            outcome.passwords_left_out
        ));
    }
    app.set_status(Status::CommandOutput(format!("NetBox: {outcome}")));
}

/// `given` made absolute: `~` expanded, then taken from a file panel's
/// directory when relative.
fn resolve(app: &App, given: &Path) -> PathBuf {
    let expanded = crate::command::expand_tilde(given);
    if expanded.is_absolute() {
        return expanded;
    }
    files_panel(app)
        .map_or_else(
            || app.get_active_panel_ref().path.clone(),
            |panel| panel.path.clone(),
        )
        .join(expanded)
}

/// The `.netbox` file under a file panel's cursor.
fn file_under_cursor(app: &App) -> Option<PathBuf> {
    files_panel(app)
        .and_then(Panel::selected_path)
        .filter(|path| {
            path.extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case(EXTENSION))
        })
}

/// The panel whose cursor the user can see on a file: the active one when it
/// lists files, else the other — the F11 row is often used while the
/// sessions list hides the file list beneath it.
fn files_panel(app: &App) -> Option<&Panel> {
    [app.get_active_panel_ref(), app.get_inactive_panel_ref()]
        .into_iter()
        .find(|panel| matches!(panel.kind, PanelKind::Files))
}

#[cfg(test)]
mod tests;
