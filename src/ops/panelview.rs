//! Purpose: The view modes (`Ctrl+1` … `Ctrl+0`) — switch the active panel's
//! column layout and name the one it is in.
//!
//! One entry point rather than an assignment in the dispatcher, so that what
//! a mode switch entails — today the field and a word in the status bar,
//! tomorrow the fullscreen flag from `Settings.panel_modes` and a scroll
//! reset — has one place to grow.

use crate::models::{App, PanelView};
use crate::status::Status;

/// Put the active panel into `view`.
pub fn set(app: &mut App, view: PanelView) {
    app.get_active_panel().view = view;
    app.set_status(Status::CommandOutput(format!("{} view", view.label())));
}

#[cfg(test)]
mod tests;
