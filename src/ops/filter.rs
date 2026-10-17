//! Purpose: File panel filter (`Ctrl+I`): open the dialog; applying it is `PanelFilter::apply`.

use crate::models::{App, Overlay};

/// Ctrl+I: open the File panel filter dialog.
pub fn open(app: &mut App) {
    app.set_overlay(Overlay::Filter(crate::models::FilterPrompt::default()));
}
