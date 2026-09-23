//! Purpose: Draws the User menu dialog (F2). A placeholder frame until the
//! feature lands; the state parameter is the one the real dialog will read.

use ratatui::Frame;

pub(super) fn render(
    f: &mut Frame,
    tracker: &mut crate::mouse::DialogMouse,
    _dialog: &crate::models::UserMenu,
) {
    super::stub::render_coming(f, tracker, "User menu");
}
