//! Purpose: The database explorer's keys. Every key reaches the dialog while it
//! is open. The keymap's `database_explorer` context names the command, which
//! this applies to the explorer and queues any request it asks for. A key nothing
//! is bound to is swallowed.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::action::database::{resolve, DatabaseCommand};
use crate::models::database::{Back, DatabaseExplorer, Requests};
use crate::models::{App, Overlay};

/// Returns `true`: the explorer consumes every key while it is open.
pub(super) fn process(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> bool {
    let Some(command) = resolve(
        &app.keymap.database_explorer,
        KeyEvent::new(code, modifiers),
    ) else {
        return true;
    };
    let Overlay::Database(explorer) = &mut app.overlay else {
        return false;
    };
    let close = apply(explorer, &mut app.database_requests, command);
    if close {
        app.close_overlay();
    }
    true
}

/// Applies `command` from an automation invocation, as a key would.
pub fn invoke(app: &mut App, command: DatabaseCommand) {
    let Overlay::Database(explorer) = &mut app.overlay else {
        return;
    };
    let close = apply(explorer, &mut app.database_requests, command);
    if close {
        app.close_overlay();
    }
}

/// Applies `command`; `true` when the dialog should close.
fn apply(explorer: &mut DatabaseExplorer, queue: &mut Requests, command: DatabaseCommand) -> bool {
    match command {
        DatabaseCommand::Up => explorer.move_up(),
        DatabaseCommand::Down => explorer.move_down(),
        DatabaseCommand::Expand => explorer.expand(queue),
        DatabaseCommand::Collapse => explorer.collapse(),
        DatabaseCommand::Open => explorer.open(queue),
        DatabaseCommand::Columns => explorer.columns(queue),
        DatabaseCommand::PageUp => explorer.page_previous(queue),
        DatabaseCommand::PageDown => explorer.page_next(queue),
        DatabaseCommand::Reload => explorer.reload(queue),
        DatabaseCommand::Back => return explorer.back() == Back::Close,
        DatabaseCommand::Close => return true,
    }
    false
}

#[cfg(test)]
mod tests;
