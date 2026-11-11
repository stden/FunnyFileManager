//! Purpose: Keys for the History dialog (Alt+F8). Esc and F10 close it; the
//! rest arrives with the feature.

use crossterm::event::{KeyCode, KeyModifiers};

use crate::models::App;

pub(super) fn process(app: &mut App, code: KeyCode, _modifiers: KeyModifiers) -> bool {
    super::stub::close_on_escape(app, code)
}

#[cfg(test)]
mod tests;
