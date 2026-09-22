//! Purpose: Task list (`Ctrl+W`): the processes this program started.

use crate::models::{App, Overlay};

/// Ctrl+W: open the Task list dialog.
pub fn open(app: &mut App) {
    app.set_overlay(Overlay::Tasks(Box::default()));
}
