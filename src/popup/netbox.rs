//! Purpose: What the two `NetBox` panels — the sessions list and a remote
//! host — share: the keys that move a list's cursor, and what a panel does
//! with a key it does not handle itself.
//!
//! Both panels are drawn over the file list, and the file list is still
//! there underneath: a key that fell through would mark, rename, wipe or
//! make a folder in a directory the user cannot see. So a key a panel does
//! not claim is resolved the way the dispatcher would resolve it, and when
//! the action would reach for the hidden entries the panel consumes it and
//! says why. Keys that belong to the program — `Tab`, `F10`, `Alt+F1`, the
//! command line — still fall through.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::action::{resolve, ActionScope};
use crate::models::{App, RowMotion};
use crate::status::Status;

/// The list key `key` is, for a panel `page` rows high. Only bare keys:
/// `Ctrl` and `Alt` chords mean something else everywhere.
pub(super) const fn row_motion(key: KeyEvent, page: usize) -> Option<RowMotion> {
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

/// Whether `key` was consumed for acting on the hidden file list, with
/// `explanation` on the status bar. A program key is handed back.
pub(super) fn refuse_entry_key(app: &mut App, key: KeyEvent, explanation: &str) -> bool {
    if resolve(key).scope() == ActionScope::Program {
        return false;
    }
    app.set_status(Status::CommandOutput(explanation.to_owned()));
    true
}

/// Whether `key` is `code` with no modifier held at all.
pub(super) fn is_bare(key: KeyEvent, code: KeyCode) -> bool {
    key.code == code && key.modifiers.is_empty()
}

#[cfg(test)]
mod tests;
