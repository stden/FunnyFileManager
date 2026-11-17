//! Purpose: Keys for the Archive commands menu (Shift+F3): the cursor walks
//! Far's rows, and Enter — or a row's own letter — runs the chosen one on the
//! archive under the panel cursor. What each row does is `ops::archive`'s.

use crossterm::event::{KeyCode, KeyModifiers};

use crate::models::{App, ArchiveCommand, Overlay};

pub(super) fn process(app: &mut App, code: KeyCode, _modifiers: KeyModifiers) -> bool {
    let Overlay::ArchiveCommands(menu) = &mut app.overlay else {
        return false;
    };
    match code {
        KeyCode::Esc | KeyCode::F(10) => app.close_overlay(),
        KeyCode::Up => menu.move_up(),
        KeyCode::Down => menu.move_down(),
        KeyCode::Home => menu.move_home(),
        KeyCode::End => menu.move_end(),
        KeyCode::Enter => {
            let chosen = menu.command();
            crate::ops::archive::run_command(app, chosen);
        }
        KeyCode::Char(letter) => {
            if let Some(chosen) = ArchiveCommand::by_hotkey(letter) {
                crate::ops::archive::run_command(app, chosen);
            }
        }
        _ => {}
    }
    true
}

#[cfg(test)]
mod tests;
