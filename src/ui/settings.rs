//! Purpose: Draws the settings dialog (the Options menu). A placeholder frame until the
//! feature lands; the state parameter is the one the real dialog will read.

use ratatui::Frame;

pub(super) fn render(
    f: &mut Frame<'_>,
    tracker: &mut crate::mouse::DialogMouse,
    dialog: &crate::models::SettingsDialog,
) {
    super::stub::render_coming(f, tracker, dialog.page.title());
}
