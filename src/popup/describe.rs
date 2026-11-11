//! Purpose: Keys for the Describe files dialog (Ctrl+Z): type the text, Enter
//! writes it and moves to the next marked entry, Esc drops the rest.

use crossterm::event::{KeyCode, KeyModifiers};

use crate::models::{App, Overlay};

pub(super) fn process(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> bool {
    let Overlay::Describe(prompt) = &mut app.overlay else {
        return false;
    };
    match code {
        KeyCode::Esc | KeyCode::F(10) => app.close_overlay(),
        KeyCode::Tab => prompt.toggle_button(),
        KeyCode::Enter if prompt.on_cancel() => app.close_overlay(),
        KeyCode::Enter => crate::ops::describe::accept(app),
        // On a button, Left and Right walk the buttons, as Far's do; in the
        // field they move the caret — see `edit`.
        KeyCode::Left | KeyCode::Right if prompt.on_cancel() => prompt.toggle_button(),
        _ => {
            super::text_field::edit(prompt.line_mut(), code, modifiers, &mut app.clipboard);
        }
    }
    true
}

#[cfg(test)]
mod tests;
