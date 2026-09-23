//! Purpose: Open the read-only language-server inventory using the scrolling report viewer.

use crate::models::{App, Overlay, Viewer};

pub(super) fn open(app: &mut App) {
    let lines = crate::lsp::inventory(&app.config.lsp, &app.get_active_panel_ref().path);
    app.set_overlay(Overlay::Viewer(Viewer::report("Language servers", lines)));
}

#[cfg(test)]
mod tests;
