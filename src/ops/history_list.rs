//! Purpose: History (`Alt+F8`): open the command history as a list.

use crate::models::{App, Overlay};

/// Alt+F8: open the History dialog.
pub fn open(app: &mut App) {
    app.set_overlay(Overlay::CommandHistory(
        crate::models::HistoryList::default(),
    ));
}
