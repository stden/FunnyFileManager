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
        // The arrows belong to the control under the cursor: they pick
        // an option on the radio group and the drop-down, and do
        // nothing in the text field, where Far uses them to move the
        // caret rather than to change the answer.
        KeyCode::Right => prompt.cycle(true),
        KeyCode::Left => prompt.cycle(false),
        KeyCode::Char(' ') if prompt.field.is_checkbox() => prompt.toggle(),
        KeyCode::Backspace if prompt.field == CopyField::Destination => {
            prompt.destination.pop();
        }
        KeyCode::Enter => submit(app),
        KeyCode::Char(ch)
            if prompt.field == CopyField::Destination
                && !modifiers.contains(KeyModifiers::CONTROL)
                && !modifiers.contains(KeyModifiers::ALT) =>
        {
            prompt.destination.push(ch);
        }
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
