//! Purpose: Keys for the Space analysis dialog (Commands): the arrows and the
//! paging keys move the cursor, Enter selects the entry in the active panel and
//! closes, Esc and F10 close. No key removes anything: F8 stays the user's.

use crossterm::event::{KeyCode, KeyModifiers};

use crate::models::space_analysis::PAGE;
use crate::models::{App, Overlay};

pub(super) fn process(app: &mut App, code: KeyCode, _modifiers: KeyModifiers) -> bool {
    let Overlay::SpaceAnalysis(dialog) = &mut app.overlay else {
        return false;
    };
    let cursor = dialog.cursor;
    match code {
        KeyCode::Esc | KeyCode::F(10) => app.close_overlay(),
        KeyCode::Up => dialog.move_to(cursor.saturating_sub(1)),
        KeyCode::Down => dialog.move_to(cursor.saturating_add(1)),
        KeyCode::PageUp => dialog.move_to(cursor.saturating_sub(PAGE)),
        KeyCode::PageDown => dialog.move_to(cursor.saturating_add(PAGE)),
        KeyCode::Home => dialog.move_to(0),
        KeyCode::End => dialog.move_to(usize::MAX),
        KeyCode::Enter => select_in_panel(app),
        _ => {}
    }
    true
}

/// Enter: put the active panel's cursor on the row's entry and close. Nothing is
/// removed; the user decides what to do with the entry, F8 included.
fn select_in_panel(app: &mut App) {
    let Overlay::SpaceAnalysis(dialog) = &app.overlay else {
        return;
    };
    let name = dialog.selected_name().map(str::to_owned);
    app.close_overlay();
    if let Some(name) = name {
        app.get_active_panel().select_named(&name);
    }
}

#[cfg(test)]
mod tests;
