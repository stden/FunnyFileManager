//! Purpose: Keys for the Extract files dialog (Shift+F2): typing edits the
//! destination, Tab and the arrows walk the controls, Enter extracts unless
//! the cursor is on Cancel, Esc and F10 take the dialog down.
//!
//! Typed characters always go to the destination field, whichever control
//! carries the cursor: the field is the only thing here that can be typed
//! into, so a letter typed with the cursor on a button is a slip, not a
//! command, and losing it would be the surprising outcome.

use crossterm::event::{KeyCode, KeyModifiers};

use crate::models::{App, Overlay};

pub(super) fn process(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> bool {
    let Overlay::ExtractPrompt(prompt) = &mut app.overlay else {
        return false;
    };
    match code {
        KeyCode::Esc | KeyCode::F(10) => app.close_overlay(),
        KeyCode::Tab | KeyCode::Down | KeyCode::Right => prompt.advance(true),
        KeyCode::BackTab | KeyCode::Up | KeyCode::Left => prompt.advance(false),
        KeyCode::Backspace => prompt.backspace(),
        KeyCode::Enter if prompt.on_cancel() => app.close_overlay(),
        KeyCode::Enter => crate::ops::archive::run_extract(app),
        KeyCode::Char(ch)
            if !modifiers.contains(KeyModifiers::CONTROL)
                && !modifiers.contains(KeyModifiers::ALT) =>
        {
            prompt.push(ch);
        }
        _ => {}
    }
    true
}

#[cfg(test)]
mod tests;
