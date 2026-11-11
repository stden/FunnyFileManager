//! Purpose: Keys for the Install MCP server dialog (F9 Commands) — the scope
//! radio, the client checkboxes and the two buttons.
//!
//! The same shape as the Add to archive dialog's keys: Tab walks the ring,
//! the arrows belong to the control under the cursor, Enter does what the
//! focused button says. Surveying and registering are `ops::mcp_install`'s.

use crossterm::event::{KeyCode, KeyModifiers};

use crate::models::{App, McpInstallField, Overlay};
use crate::ops::mcp_install;
use crate::search::Direction;

pub(super) fn process(app: &mut App, code: KeyCode, _modifiers: KeyModifiers) -> bool {
    let Overlay::McpInstall(dialog) = &mut app.overlay else {
        return false;
    };
    let on_radio = dialog.field == McpInstallField::Scope;
    match code {
        KeyCode::Esc | KeyCode::F(10) => app.close_overlay(),
        KeyCode::Tab | KeyCode::Down => dialog.advance(Direction::Forward),
        KeyCode::BackTab | KeyCode::Up => dialog.advance(Direction::Backward),
        KeyCode::Right if on_radio => mcp_install::switch_scope(app, Direction::Forward),
        KeyCode::Left if on_radio => mcp_install::switch_scope(app, Direction::Backward),
        KeyCode::Char(' ') => dialog.toggle(),
        KeyCode::Enter => submit(app),
        _ => {}
    }
    true
}

/// Enter: Cancel cancels, and so does a dialog with no server to register;
/// anywhere else it is the default button, Register.
fn submit(app: &mut App) {
    let Overlay::McpInstall(dialog) = &app.overlay else {
        return;
    };
    if dialog.field == McpInstallField::Cancel || !dialog.can_register() {
        app.close_overlay();
        return;
    }
    mcp_install::submit(app);
}

#[cfg(test)]
mod tests;
