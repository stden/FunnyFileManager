//! Purpose: Folders history (`Alt+F12`): open the list of folders visited.

use crate::models::{App, Overlay};

/// Alt+F12: open the Folders history dialog.
pub fn open(app: &mut App) {
    app.set_overlay(Overlay::FolderHistory(crate::models::HistoryList::default()));
}
