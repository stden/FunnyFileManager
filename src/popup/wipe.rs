//! Purpose: Keys for the Wipe dialog (Alt+Del) — the answers F8's delete
//! dialog takes, so a hand trained on one is never surprised by the other.
//! What a Yes does lives in `ops::wipe`.

use crossterm::event::{KeyCode, KeyModifiers};

use crate::models::App;

pub(super) fn process(app: &mut App, code: KeyCode, _modifiers: KeyModifiers) -> bool {
    match code {
        KeyCode::Char('y' | 'Y') | KeyCode::Enter => crate::ops::wipe::run(app),
        KeyCode::Char('n' | 'N') | KeyCode::Esc | KeyCode::F(10) => crate::ops::wipe::cancel(app),
        _ => {}
    }
    true
}

#[cfg(test)]
mod tests;
