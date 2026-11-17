//! Purpose: Keys for the session dialog (F4 / Shift+F4 on the sessions
//! panel): Tab walks the controls, typing edits the text field with the
//! cursor, Left / Right / Space change a choice, Enter stores the session.
//! Esc and F10 close it without storing anything.

use crossterm::event::{KeyCode, KeyModifiers};

use crate::models::{App, Overlay, SessionField};
use crate::ops;

pub(super) fn process(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> bool {
    let Overlay::SessionEdit(dialog) = &mut app.overlay else {
        return false;
    };
    let on_choice = matches!(dialog.field(), SessionField::Protocol | SessionField::Auth);
    match code {
        KeyCode::Esc | KeyCode::F(10) => app.close_overlay(),
        KeyCode::Tab | KeyCode::Down => dialog.tab(true),
        KeyCode::BackTab | KeyCode::Up => dialog.tab(false),
        KeyCode::Enter if dialog.on_cancel() => app.close_overlay(),
        KeyCode::Enter => save(app),
        // Left and Right change a choice and do nothing in a text field;
        // Space changes a choice but is typed into a text field.
        KeyCode::Left => dialog.cycle_choice(false),
        KeyCode::Right => dialog.cycle_choice(true),
        KeyCode::Char(' ') if on_choice => dialog.cycle_choice(true),
        KeyCode::Char(ch)
            if !modifiers.intersects(KeyModifiers::CONTROL.union(KeyModifiers::ALT)) =>
        {
            dialog.push(ch);
        }
        KeyCode::Backspace => dialog.backspace(),
        _ => {}
    }
    true
}

/// Enter: store the session. On an error the dialog stays open with the
/// reason on the status bar, so the field is corrected, not retyped.
fn save(app: &mut App) {
    if let Overlay::SessionEdit(dialog) = &app.overlay {
        let dialog = dialog.as_ref().clone();
        let outcome = ops::sessions::save_edit(app, &dialog);
        ops::sessions::report(app, outcome);
    }
}

#[cfg(test)]
mod tests;
