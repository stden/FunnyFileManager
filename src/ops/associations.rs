//! Purpose: File associations: the dialog, and the lookup `Enter` consults before the executable rule.

use crate::models::{App, Overlay};

/// the Commands menu: open the File associations dialog.
pub fn open(app: &mut App) {
    app.set_overlay(Overlay::Associations(Box::default()));
}

/// The command an association gives the entry under the cursor, consulted by
/// `Enter` before the executable rule. `None` until the feature lands, so
/// `Enter` keeps doing what it does today.
#[must_use]
pub fn association_command(_app: &App) -> Option<String> {
    None
}
