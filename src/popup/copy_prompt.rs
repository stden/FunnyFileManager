//! Copy and move dialog controls and acceptance.

use crate::copy::CopyField;
use crate::models::{App, Overlay};
use crossterm::event::{KeyCode, KeyModifiers};

pub(super) fn process(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> bool {
    let Overlay::CopyPrompt(prompt) = &mut app.overlay else {
        return false;
    };
    match code {
        KeyCode::Esc | KeyCode::F(10) => app.close_overlay(),
        KeyCode::Tab | KeyCode::Down => prompt.advance(true),
        KeyCode::BackTab | KeyCode::Up => prompt.advance(false),
        KeyCode::Enter => submit(app),
        // The text field takes every other key: the caret, selection,
        // typing and the clipboard, as in any Far edit control.
        _ if prompt.field == CopyField::Destination => {
            super::text_field::edit(&mut prompt.destination, code, modifiers, &mut app.clipboard);
        }
        // Elsewhere the arrows pick an option on the radio group and the
        // drop-down, and Space ticks a checkbox.
        KeyCode::Right => prompt.cycle(true),
        KeyCode::Left => prompt.cycle(false),
        KeyCode::Char(' ') if prompt.field.is_checkbox() => prompt.toggle(),
        _ => {}
    }
    true
}

fn submit(app: &mut App) {
    let Overlay::CopyPrompt(prompt) = &app.overlay else {
        return;
    };
    // Enter on Cancel means cancel, wherever else it would
    // mean go: the focused button is what Enter acts on.
    if prompt.field == CopyField::Cancel || !prompt.is_ready() {
        app.close_overlay();
    } else {
        let settings = (**prompt).clone();
        app.close_overlay();
        crate::ops::copy_with(app, &settings);
    }
}

#[cfg(test)]
mod tests;
