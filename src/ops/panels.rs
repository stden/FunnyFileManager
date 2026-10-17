//! Purpose: Panel On/Off (`Ctrl+F1` / `Ctrl+F2`, the Left / Right menu row)
//! — one panel leaves the screen and the other takes its half.
//!
//! Far leaves the freed half to the console behind the panels; a terminal
//! file manager has no console worth showing there, so here the visible
//! panel takes the whole width instead (the layout reads
//! `App::panels_on_screen`, which is why nothing here touches the renderer).
//!
//! Hiding the active panel makes the other one active, and neither `Tab`
//! nor `Left` / `Right` nor the F9 bar can make a hidden panel active
//! (`App::select_panel` is the rule they share).

use crate::models::{App, PanelSide, PanelsShown};
use crate::status::Status;

/// `Ctrl+F1` / `Ctrl+F2`: hide or show `side`.
///
/// The last panel on screen stays — `Ctrl+O` is how both panels go — and
/// the status line says so rather than letting the key look broken.
pub fn toggle(app: &mut App, side: PanelSide) {
    let shown = app.panels_shown.toggled(side);
    if shown == app.panels_shown {
        app.set_status(Status::CommandOutput(format!(
            "{} panel stays on: the last panel cannot be hidden (Ctrl+O hides both)",
            name(side)
        )));
        return;
    }
    app.panels_shown = shown;
    if !shown.shows(app.active_panel) {
        app.switch_panel();
    }
    let state = if shown.shows(side) { "on" } else { "off" };
    app.set_status(Status::CommandOutput(format!(
        "{} panel {state}",
        name(side)
    )));
}

/// `Tab`: the other panel becomes active — unless it is off screen, in which
/// case Far ignores the key and so does this.
pub fn switch_panel(app: &mut App) {
    if app.panels_shown == PanelsShown::Both {
        app.switch_panel();
    }
}

/// `Left` / `Right`: make `side` active, if it is on screen. The F9 bar's
/// Left / Right headings apply the same rule through `App::set_menu`.
pub fn select(app: &mut App, side: PanelSide) {
    app.select_panel(side);
}

/// `Ctrl+U`: swap the panels — unless one is hidden.
///
/// Far's `SwapPanels` does nothing then, and so does this: the swap would
/// put the hidden panel's directory on screen under the visible panel's name.
pub fn swap_panels(app: &mut App) {
    if let Some(hidden) = hidden(app.panels_shown) {
        app.set_status(Status::CommandOutput(format!(
            "Panels are not swapped while one is hidden ({} shows the {} panel)",
            show_key(hidden),
            name(hidden).to_lowercase()
        )));
        return;
    }
    app.swap_panels();
    app.set_status(Status::PanelsSwapped);
}

/// The panel `Ctrl+F1` / `Ctrl+F2` took off the screen, if any.
const fn hidden(shown: PanelsShown) -> Option<PanelSide> {
    match shown {
        PanelsShown::Both => None,
        PanelsShown::LeftOnly => Some(PanelSide::Right),
        PanelsShown::RightOnly => Some(PanelSide::Left),
    }
}

const fn show_key(side: PanelSide) -> &'static str {
    match side {
        PanelSide::Left => "Ctrl+F1",
        PanelSide::Right => "Ctrl+F2",
    }
}

const fn name(side: PanelSide) -> &'static str {
    match side {
        PanelSide::Left => "Left",
        PanelSide::Right => "Right",
    }
}

#[cfg(test)]
mod tests;
