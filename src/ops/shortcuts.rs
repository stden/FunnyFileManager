//! Purpose: Folder shortcuts: the ten bookmark slots as a dialog.

use crate::models::{App, Overlay};

/// the Commands menu: open the Folder shortcuts dialog.
pub fn open(app: &mut App) {
    app.set_overlay(Overlay::FolderShortcuts(
        crate::models::ShortcutList::default(),
    ));
}
