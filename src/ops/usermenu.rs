//! Purpose: The user menu (`F2`) and its editor (Commands → Edit user menu).

use crate::models::{App, Overlay};

/// F2: open the User menu dialog.
pub fn open(app: &mut App) {
    app.set_overlay(Overlay::UserMenu(Box::default()));
}

/// Commands → Edit user menu: opens `menu.ini` in the editor once the feature
/// lands; until then the status bar says so.
pub fn edit(app: &mut App) {
    super::stub::report_coming(app, "Edit user menu");
}

#[cfg(test)]
mod tests;
