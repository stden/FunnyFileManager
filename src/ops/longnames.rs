//! Purpose: Show long names (`Ctrl+N`) — the toggle between a name as it is
//! and Far's 8.3 form of it.
//!
//! Only the flag lives here. The 8.3 forms themselves come from
//! [`crate::models::ShortNames`], which the panel renderer builds from the
//! listing whenever short names are on; the menu ticks the row while they are
//! off.

use crate::models::App;
use crate::status::Status;

/// `Ctrl+N`: flip the active panel between long and short names.
pub fn toggle(app: &mut App) {
    let panel = app.get_active_panel();
    panel.options.short_names = !panel.options.short_names;
    let report = if panel.options.short_names {
        "Short names"
    } else {
        "Long names"
    };
    app.set_status(Status::CommandOutput(report.to_owned()));
}

#[cfg(test)]
mod tests;
