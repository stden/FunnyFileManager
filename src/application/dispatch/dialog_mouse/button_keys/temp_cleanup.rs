//! Purpose: Cleanup clicks focus the same typed answer that Enter activates.
use crate::models::temp_cleanup::{TempChoice, TempCleanupState};
use crate::models::{Confirm, TempCleanupDialog};
use crossterm::event::{KeyCode, KeyEvent};

pub(super) fn button(dialog: &mut TempCleanupDialog, index: usize) -> Option<KeyEvent> {
    match &mut dialog.state {
        TempCleanupState::Preview { plan, focus } => {
            *focus = match index {
                0 if !plan.files.is_empty() => TempChoice::Clean,
                1 => TempChoice::Cancel,
                _ => return None,
            };
        }
        TempCleanupState::Confirm { answer, .. } => {
            *answer = match index {
                0 => Confirm::Yes,
                1 => Confirm::No,
                _ => return None,
            };
        }
        _ if index == 0 => return Some(KeyCode::Esc.into()),
        _ => return None,
    }
    Some(KeyCode::Enter.into())
}
