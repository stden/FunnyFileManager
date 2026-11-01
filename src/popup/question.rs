//! Purpose: Keys of the yes/no question: Left, Right and Tab move between
//! the answers, `Y` / `N` answer outright, Enter takes the one under the
//! cursor and hands Yes to `ops::prompts`.

use crossterm::event::{KeyCode, KeyModifiers};

use crate::models::{App, Confirm, Overlay};

pub(super) fn process(app: &mut App, code: KeyCode, _modifiers: KeyModifiers) -> bool {
    let Overlay::Question(question) = &mut app.overlay else {
        return false;
    };
    match code {
        KeyCode::Left | KeyCode::Right | KeyCode::Tab | KeyCode::BackTab => question.toggle(),
        KeyCode::Char('y' | 'Y') => answer(app, Confirm::Yes),
        KeyCode::Char('n' | 'N') | KeyCode::Esc | KeyCode::F(10) => answer(app, Confirm::No),
        KeyCode::Enter => {
            let chosen = question.answer();
            answer(app, chosen);
        }
        _ => {}
    }
    true
}

/// The dialog closes before Yes acts, so what it does can put up a dialog
/// of its own.
fn answer(app: &mut App, chosen: Confirm) {
    if let Overlay::Question(question) = std::mem::take(&mut app.overlay) {
        if chosen.is_yes() {
            crate::ops::prompts::confirm_question(app, question.into_purpose());
        }
    }
}

#[cfg(test)]
mod tests;
