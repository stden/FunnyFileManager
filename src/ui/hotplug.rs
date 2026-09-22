//! Purpose: Draws the Hotplug devices list dialog (the Commands menu). A placeholder frame until the
//! feature lands; the state parameter is the one the real dialog will read.

use ratatui::Frame;

pub(super) fn render(
    f: &mut Frame,
    tracker: &mut crate::mouse::DialogMouse,
    _dialog: &crate::models::DeviceList,
) {
    super::stub::render_coming(f, tracker, "Hotplug devices list");
}
