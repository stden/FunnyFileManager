//! Purpose: Keys for the Find duplicate blocks dialog. Esc and F10 close it;
//! Enter starts the search, or closes the dialog on Cancel.

use crossterm::event::{KeyCode, KeyModifiers};

use crate::models::{App, Overlay};

pub(super) fn process(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> bool {
    let Overlay::Duplicates(dialog) = &mut app.overlay else {
        return false;
    };
    match code {
        KeyCode::Esc | KeyCode::F(10) => app.close_overlay(),
        KeyCode::Tab | KeyCode::Down => dialog.tab(true),
        KeyCode::BackTab | KeyCode::Up => dialog.tab(false),
        KeyCode::Enter if dialog.on_cancel() => app.close_overlay(),
        KeyCode::Enter => crate::ops::duplicates::accept(app),
        KeyCode::Left | KeyCode::Right if dialog.on_button() => dialog.toggle_button(),
        _ => {
            if let Some(line) = dialog.active_line_mut() {
                super::text_field::edit(line, code, modifiers, &mut app.clipboard);
            }
        }
    }
    true
}

#[cfg(test)]
mod tests;
