//! Purpose: Screens list (`F12`): the screens this program has.

use crate::models::{App, Overlay};

/// F12: open the Screens dialog.
pub fn open(app: &mut App) {
    app.set_overlay(Overlay::Screens(crate::models::ScreenList::default()));
}
