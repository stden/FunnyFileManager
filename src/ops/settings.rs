//! Purpose: The Options menu: one settings dialog per page.

use crate::models::{App, Overlay, SettingsDialog};

/// Open the settings dialog on `page` — one row of the Options menu.
pub fn open(app: &mut App, page: crate::menu::SettingsPage) {
    app.set_overlay(Overlay::Settings(Box::new(SettingsDialog::for_page(page))));
}
