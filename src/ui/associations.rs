//! Purpose: Draws the File associations dialog (the Commands menu). A placeholder frame until the
//! feature lands; the state parameter is the one the real dialog will read.

use ratatui::Frame;

pub(super) fn render(
    f: &mut Frame<'_>,
    tracker: &mut crate::mouse::DialogMouse,
    _dialog: &crate::models::AssociationList,
) {
    super::stub::render_coming(f, tracker, "File associations");
}
