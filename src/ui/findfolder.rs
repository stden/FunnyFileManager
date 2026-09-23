//! Purpose: Draws the Find folder dialog (Alt+F10). A placeholder frame until the
//! feature lands; the state parameter is the one the real dialog will read.

use ratatui::Frame;

pub(super) fn render(
    f: &mut Frame,
    tracker: &mut crate::mouse::DialogMouse,
    _dialog: &crate::models::FolderTree,
) {
    super::stub::render_coming(f, tracker, "Find folder");
}
