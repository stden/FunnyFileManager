//! Purpose: Keys for the File attributes dialog (Ctrl+A).
//!
//! Tab and the vertical arrows walk the ring; Space answers the box under the
//! cursor; text lands in the time field that has it; Enter is Set from
//! anywhere but Cancel. The horizontal arrows step along a row of boxes,
//! because three boxes on one line invite them, and do nothing in a time
//! field, where Far uses them for the caret rather than the answer.

use crossterm::event::{KeyCode, KeyModifiers};

use crate::models::{App, AttributesField, Overlay};
use crate::status::Status;

pub(super) fn process(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> bool {
    let Overlay::Attributes(dialog) = &mut app.overlay else {
        return false;
    };
    match code {
        KeyCode::Esc | KeyCode::F(10) => app.close_overlay(),
        KeyCode::Tab | KeyCode::Down => dialog.advance(true),
        KeyCode::BackTab | KeyCode::Up => dialog.advance(false),
        KeyCode::Right if matches!(dialog.field, AttributesField::Flag(_)) => dialog.advance(true),
        KeyCode::Left if matches!(dialog.field, AttributesField::Flag(_)) => dialog.advance(false),
        KeyCode::Char(' ') if !is_time_field(dialog.field) => press_space(app),
        KeyCode::Backspace => dialog.backspace(),
        KeyCode::Enter => submit(app),
        KeyCode::Char(ch)
            if !modifiers.contains(KeyModifiers::CONTROL)
                && !modifiers.contains(KeyModifiers::ALT) =>
        {
            dialog.push(ch);
        }
        _ => {}
    }
    true
}

const fn is_time_field(field: AttributesField) -> bool {
    matches!(field, AttributesField::Modified | AttributesField::Accessed)
}

/// Space on a box flips it; on a button it presses it, as Far's does.
fn press_space(app: &mut App) {
    let Overlay::Attributes(dialog) = &mut app.overlay else {
        return;
    };
    match dialog.field {
        AttributesField::Set | AttributesField::Cancel => submit(app),
        _ => dialog.toggle(),
    }
}

/// Enter: Cancel where the cursor is on it, Set everywhere else.
///
/// A time that does not parse keeps the dialog open with the reason in the
/// status bar: the user is one Backspace from fixing it, and closing would
/// throw away every box they moved.
fn submit(app: &mut App) {
    let Overlay::Attributes(dialog) = &app.overlay else {
        return;
    };
    if dialog.field == AttributesField::Cancel {
        app.close_overlay();
        return;
    }
    if let Err(refused) = dialog.requested_times() {
        app.set_status(Status::CommandOutput(refused.to_string()));
        return;
    }
    let dialog = (**dialog).clone();
    app.close_overlay();
    crate::ops::attributes::apply(app, &dialog);
}

#[cfg(test)]
mod tests;
