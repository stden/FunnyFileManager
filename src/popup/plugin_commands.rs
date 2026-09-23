//! Purpose: Keys for Far's `F11` Plugin commands list: the list keys walk
//! the rows, Enter runs one, Esc and F10 leave.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::models::{App, Overlay};
use crate::ops::plugin_commands;
use crate::status::Status;

/// Rows a page key moves: the list is short, so a page is all of it.
const PAGE: usize = 10;

pub(super) fn process(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> bool {
    let Overlay::PluginCommands(list) = &mut app.overlay else {
        return false;
    };
    if let Some(motion) = super::netbox::row_motion(KeyEvent::new(code, modifiers), PAGE) {
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

#[cfg(test)]
mod tests;
