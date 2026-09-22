//! Purpose: Keys for the Find file dialog (Alt+F7). Esc and F10 close it;
//! Enter runs the search.

use crossterm::event::{KeyCode, KeyModifiers};

use crate::models::{App, Overlay};

pub(super) fn process(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> bool {
    let Overlay::FindFile(dialog) = &mut app.overlay else {
        return false;
    };
    match code {
        KeyCode::Esc | KeyCode::F(10) => app.close_overlay(),
        KeyCode::Tab | KeyCode::Down => dialog.tab(true),
        KeyCode::BackTab | KeyCode::Up => dialog.tab(false),
        KeyCode::Enter if dialog.on_cancel() => app.close_overlay(),
        KeyCode::Enter => crate::ops::findfile::accept(app),
        KeyCode::Left | KeyCode::Right if dialog.on_button() => dialog.toggle_button(),
        KeyCode::Char(ch) if is_text(modifiers) => dialog.push(ch),
        KeyCode::Backspace => dialog.backspace(),
        _ => {}
    }
    true
}

fn is_text(modifiers: KeyModifiers) -> bool {
    !modifiers.contains(KeyModifiers::CONTROL) && !modifiers.contains(KeyModifiers::ALT)
}

#[cfg(test)]
mod tests;
