//! Purpose: Modal keys for temp-only cleanup, with explicit confirmation before deletion.
use crate::models::temp_cleanup::TempCleanupState;
use crate::models::{App, Confirm, Overlay};
use crate::ops::system_temp_cleanup as operation;
use crossterm::event::{KeyCode, KeyModifiers};

pub(super) fn process(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> bool {
    let Overlay::TempCleanup(dialog) = &app.overlay else {
        return false;
    };
    let confirming = matches!(dialog.state, TempCleanupState::Confirm { .. });
    let plain = modifiers.is_empty() || modifiers == KeyModifiers::SHIFT;
    match code {
        KeyCode::Esc | KeyCode::F(10) => operation::cancel(app),
        KeyCode::Enter if modifiers.is_empty() => operation::accept(app),
        KeyCode::Tab | KeyCode::BackTab | KeyCode::Left | KeyCode::Right if plain => {
            operation::toggle(app);
        }
        KeyCode::Char('y' | 'Y') if confirming && plain => {
            operation::answer(app, Confirm::Yes);
        }
        KeyCode::Char('n' | 'N') if confirming && plain => {
            operation::answer(app, Confirm::No);
        }
        _ => {}
    }
    true
}
