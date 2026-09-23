//! Purpose: Find folder (`Alt+F10`): open the folder tree as a dialog.

use crate::models::{App, Overlay};

/// Alt+F10: open the Find folder dialog.
pub fn open(app: &mut App) {
    app.set_overlay(Overlay::FindFolder(Box::default()));
}
