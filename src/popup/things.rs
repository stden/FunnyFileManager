//! Purpose: Keys for the Things catalog dialog (F9 Commands). Typing filters the
//! machines, the arrows and paging move the cursor, Space marks a row, Enter
//! imports, and Esc or F10 close. Every key is taken here, so a letter typed in
//! the dialog never reaches the panels behind it.

use crossterm::event::{KeyCode, KeyModifiers};

use crate::models::things::{ThingsCatalog, ThingsState};
use crate::models::{App, Overlay, RowMotion};
use crate::ops::things as operation;

/// How far Page Up and Page Down move the cursor.
const PAGE: usize = 10;

pub(super) fn process(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> bool {
    let Overlay::Things(dialog) = &mut app.overlay else {
        return false;
    };
    match code {
        KeyCode::Esc | KeyCode::F(10) => app.close_overlay(),
        KeyCode::Enter => operation::submit(app),
        _ => {
            if let ThingsState::Ready(catalog) = &mut dialog.state {
                edit(catalog, code, modifiers);
            }
        }
    }
    true
}

/// What a key does to a shown catalog: move the cursor, mark the row under it,
/// or edit the filter.
fn edit(catalog: &mut ThingsCatalog, code: KeyCode, modifiers: KeyModifiers) {
    match code {
        KeyCode::Up => catalog.move_cursor(RowMotion::Previous),
        KeyCode::Down => catalog.move_cursor(RowMotion::Next),
        KeyCode::PageUp => catalog.move_cursor(RowMotion::PageUp(PAGE)),
        KeyCode::PageDown => catalog.move_cursor(RowMotion::PageDown(PAGE)),
        KeyCode::Home => catalog.move_cursor(RowMotion::First),
        KeyCode::End => catalog.move_cursor(RowMotion::Last),
        KeyCode::Char(' ') => catalog.toggle_mark(),
        KeyCode::Backspace => catalog.erase_filter(),
        KeyCode::Char(ch) if !modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) => {
            catalog.type_filter(ch);
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests;
