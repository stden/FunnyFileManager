//! Purpose: Keys for Far's `F11` Plugin commands list: the list keys walk
//! the rows, Enter runs one, Esc and F10 leave.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::models::{App, Overlay, RowMotion};
use crate::ops::plugin_commands;
use crate::status::Status;

/// Rows a page key moves: the list is short, so a page is all of it.
const PAGE: usize = 10;

pub(super) fn process(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> bool {
    let Overlay::PluginCommands(list) = &mut app.overlay else {
        return false;
    };
    if let Some(motion) = row_motion(KeyEvent::new(code, modifiers), PAGE) {
        list.move_cursor(motion);
        return true;
    }
    match code {
        KeyCode::Esc | KeyCode::F(10) => app.close_overlay(),
        KeyCode::Enter => {
            let command = list.selected();
            if let Err(error) = plugin_commands::run(app, command) {
                app.set_status(Status::Error(error.to_string()));
            }
        }
        _ => {}
    }
    true
}

/// The list key `key` is, for a list `page` rows high: the dialog
/// protocol's list keys, bare or shifted. `Ctrl` and `Alt` chords mean
/// something else everywhere.
const fn row_motion(key: KeyEvent, page: usize) -> Option<RowMotion> {
    if key
        .modifiers
        .intersects(KeyModifiers::CONTROL.union(KeyModifiers::ALT))
    {
        return None;
    }
    Some(match key.code {
        KeyCode::Up => RowMotion::Previous,
        KeyCode::Down => RowMotion::Next,
        KeyCode::Home => RowMotion::First,
        KeyCode::End => RowMotion::Last,
        KeyCode::PageUp => RowMotion::PageUp(page),
        KeyCode::PageDown => RowMotion::PageDown(page),
        _ => return None,
    })
}

#[cfg(test)]
mod tests;
