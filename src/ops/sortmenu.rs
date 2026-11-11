//! Purpose: Sort modes (`Ctrl+F12`): the menu, and the keys that set a mode directly.
//!
//! Every path here ends in a re-listing of the active panel: the mode and the
//! order are inputs to the listing, and the only way to see their effect is
//! to load the directory again. One function does that, so a mode chosen
//! from the menu, from `Ctrl+F6` and from `:sort` cannot drift apart.

use std::io;

use crate::models::{App, Overlay, SortMenu, SortMode, SortOrder, SortRow};
use crate::status::Status;

/// Ctrl+F12, and the Left / Right menu's "Sort modes" row: open the Sort by
/// menu on the panel's current mode.
pub fn open(app: &mut App) {
    let side = app.active_panel;
    let panel = app.get_active_panel_ref();
    let menu = SortMenu::open(panel.sort_mode, panel.sort_order, side);
    app.set_overlay(Overlay::SortModes(menu));
}

/// `Ctrl+F3` … `Ctrl+F11`, and Enter on a mode row: set the mode.
///
/// Far's rule: the mode already in force, chosen again, runs the other way;
/// any other mode starts the direct way, as `:sort` leaves it.
pub fn set(app: &mut App, mode: SortMode) {
    let panel = app.get_active_panel();
    if panel.sort_mode == mode {
        panel.sort_order = panel.sort_order.toggled();
    } else {
        panel.sort_mode = mode;
        panel.sort_order = SortOrder::Direct;
    }
    refresh(app);
}

/// Enter in the menu.
pub fn choose(app: &mut App, row: SortRow) {
    match row {
        SortRow::Mode(mode) => set(app, mode),
        SortRow::Reverse => toggle_order(app),
    }
}

/// `+` and `-` in the menu: an order, whatever it was before.
pub fn order(app: &mut App, order: SortOrder) {
    app.get_active_panel().sort_order = order;
    refresh(app);
}

/// `*` in the menu, and the "Reverse order" row.
pub fn toggle_order(app: &mut App) {
    let panel = app.get_active_panel();
    panel.sort_order = panel.sort_order.toggled();
    refresh(app);
}

/// Load the directory under the new rule and say what the panel is now
/// sorted by. A failure leaves the old entries in place.
pub(super) fn relist(app: &mut App) -> io::Result<()> {
    let panel = app.get_active_panel();
    panel.load_entries()?;
    panel.ensure_selection();
    app.set_status(Status::Sort(describe(app)));
    Ok(())
}

/// A menu key has nowhere to return an error to, so the status bar names it.
fn refresh(app: &mut App) {
    if let Err(error) = relist(app) {
        app.set_status(Status::Error(error.to_string()));
    }
}

/// "Name", or "Name, reverse order" — the mode label the status line already
/// knows, with the order only when it is worth a word.
fn describe(app: &App) -> String {
    let panel = app.get_active_panel_ref();
    let label = panel.sort_mode.label();
    if panel.sort_order.is_reversed() {
        format!("{label}, reverse order")
    } else {
        label.to_owned()
    }
}

#[cfg(test)]
mod tests;
