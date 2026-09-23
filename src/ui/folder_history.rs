//! Purpose: Draws the Folders history dialog (Alt+F12). A placeholder frame until the
//! feature lands; the state parameter is the one the real dialog will read.

use ratatui::Frame;

pub(super) fn render(
    f: &mut Frame,
    tracker: &mut crate::mouse::DialogMouse,
    _dialog: &crate::models::HistoryList,
) {
    super::stub::render_coming(f, tracker, "Folders history");
}
