//! Purpose: Keys for the Add to archive dialog (Shift+F1) — the name field, the
//! format radio, the delete checkbox and the two buttons.
//!
//! The same shape as the copy dialog's keys: Tab walks the ring, the arrows
//! belong to the control under the cursor, Enter does what the focused
//! button says. The archive itself is `ops::archive`'s to write.

use crossterm::event::{KeyCode, KeyModifiers};

use crate::models::{App, ArchiveField, Overlay};

pub(super) fn process(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> bool {
    let Overlay::ArchivePrompt(prompt) = &mut app.overlay else {
        return false;
    };
    match code {
        KeyCode::Esc | KeyCode::F(10) => app.close_overlay(),
        KeyCode::Tab | KeyCode::Down => prompt.advance(true),
        KeyCode::BackTab | KeyCode::Up => prompt.advance(false),
        KeyCode::Enter => submit(app),
        // The name field takes every other key: caret, selection, typing
        // and the clipboard.
        _ if prompt.field == ArchiveField::Name => {
            super::text_field::edit(&mut prompt.name, code, modifiers, &mut app.clipboard);
        }
        // On the radio the arrows pick a format.
        KeyCode::Right => prompt.cycle(true),
        KeyCode::Left => prompt.cycle(false),
        KeyCode::Char(' ') if prompt.field.is_checkbox() => prompt.toggle(),
        _ => {}
    }
    true
}

fn submit(app: &mut App) {
    let Overlay::ArchivePrompt(prompt) = &app.overlay else {
        return;
    };
    // Enter on Cancel means cancel, wherever else it would mean go: the
    // focused button is what Enter acts on.
    if prompt.field == ArchiveField::Cancel || !prompt.is_ready() {
        app.close_overlay();
        return;
    }
    let prompt = (**prompt).clone();
    app.close_overlay();
    crate::ops::archive::add(app, &prompt);
}

#[cfg(test)]
mod tests;
