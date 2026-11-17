//! Purpose: F9 Commands > Things catalog: the dialog, its read, and the import.
//!
//! The dialog opens on the settings the environment holds. The chosen machines
//! are imported through the same store as the `NetBox` import.

use std::sync::Arc;

use crate::models::things::{ThingsCatalog, ThingsDialog, ThingsState};
use crate::models::{App, Overlay};
use crate::ops::sessions::import::adopt;
use crate::sessions::import::ImportReport;
use crate::status::Status;
use crate::things::{self, Config, ConfigError, Listing, ThingsError};

#[cfg(test)]
mod tests;

/// F9 > Things catalog: open the dialog. The read is queued when the settings
/// the environment holds are good, and the dialog says what to set when not.
pub fn open(app: &mut App) {
    open_with(app, Config::from_env());
}

/// Open the dialog on `config`.
pub fn open_with(app: &mut App, config: Result<Config, ConfigError>) {
    let state = match config {
        Ok(config) => ThingsState::Queued(config),
        Err(error) => ThingsState::Setup(error.to_string()),
    };
    app.set_overlay(Overlay::Things(Box::new(ThingsDialog::new(state))));
}

/// A read's answer, into the dialog that asked for it. A dialog closed or
/// reopened since is left as it is.
pub fn publish(app: &mut App, generation: &Arc<()>, answer: Result<Listing, ThingsError>) {
    let Overlay::Things(dialog) = &mut app.overlay else {
        return;
    };
    if !Arc::ptr_eq(&dialog.generation, generation) {
        return;
    }
    match answer {
        Ok(listing) => {
            let catalog = ThingsCatalog::new(listing);
            let line = loaded_line(&catalog);
            dialog.state = ThingsState::Ready(catalog);
            app.set_status(Status::CommandOutput(line));
        }
        Err(error) => dialog.state = ThingsState::Failed(error.message().to_owned()),
    }
}

/// The status line for a loaded catalog: its count, and the server's total
/// when that is larger.
fn loaded_line(catalog: &ThingsCatalog) -> String {
    let count = catalog.machine_count();
    match catalog.unshown_total() {
        Some(total) => format!("Things: showing {count} of {total} machines"),
        None => format!("Things: {count} machines"),
    }
}

/// Enter: import what the dialog has chosen and close it. A dialog that is not
/// showing machines just closes; one with nothing to import says so.
pub fn submit(app: &mut App) {
    let Overlay::Things(dialog) = &app.overlay else {
        return;
    };
    let ThingsState::Ready(catalog) = &dialog.state else {
        app.close_overlay();
        return;
    };
    let chosen = catalog.chosen();
    if chosen.is_empty() {
        app.close_overlay();
        app.set_status(Status::Error("Things: no machines to import".to_owned()));
        return;
    }
    let imported = things::sessions_from(chosen);
    match adopt(app, imported) {
        Ok(report) => {
            app.close_overlay();
            announce(app, &report);
        }
        Err(error) => app.set_status(Status::Error(format!("Things: {error}"))),
    }
}

/// Each skipped machine and its reason go to the console transcript; the
/// counts go to the status line.
fn announce(app: &mut App, report: &ImportReport) {
    for skipped in &report.skipped {
        app.console
            .record_output(&format!("Things: skipped {skipped}"));
    }
    app.set_status(Status::CommandOutput(format!("Things: {report}")));
}
