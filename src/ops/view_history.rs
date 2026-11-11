//! Purpose: File view history (`Alt+F11`): open the list of files viewed and edited.

use crate::models::{App, Overlay};

/// Alt+F11: open the File view history dialog.
pub fn open(app: &mut App) {
    app.set_overlay(Overlay::ViewHistory(crate::models::HistoryList::default()));
}
