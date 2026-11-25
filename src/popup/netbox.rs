//! Purpose: What the two `NetBox` panels — the sessions list and a remote
//! host — share: what a panel does with a key, or an action, it does not own.
//!
//! Both panels are drawn over the file list, and the file list is still
//! there underneath: a key that fell through would mark, rename, wipe or
//! make a folder in a directory the user cannot see. So a key a panel does
//! not claim is resolved the way the dispatcher would resolve it, and when
//! the action would reach for the hidden entries the panel consumes it and
//! says why. A menu row's action is asked the same. Keys and actions that
//! belong to the program — `Tab`, `F10`, `Alt+F1`, the command line — still
//! fall through. Which keys a panel owns is the keymap's `netbox_panel`.

use crossterm::event::KeyEvent;

use crate::action::{resolve, Action, ActionScope};
use crate::models::App;
use crate::status::Status;

/// What a menu row that would act on the hidden files is told.
pub(super) const HIDDEN_ENTRIES: &str =
    "NetBox: this command works on files; press Enter on `..` to return to them";

/// Whether `key` was consumed for acting on the hidden file list, with
/// `explanation` on the status bar. A program key is handed back.
pub(super) fn refuse_entry_key(app: &mut App, key: KeyEvent, explanation: &str) -> bool {
    let action = resolve(&app.keymap.panels, key);
    refuse_entry(app, action, explanation)
}

/// [`refuse_entry_key`] for an action that arrived without a key.
pub(super) fn refuse_entry(app: &mut App, action: Action, explanation: &str) -> bool {
    if action.scope() == ActionScope::Program {
        return false;
    }
    app.set_status(Status::CommandOutput(explanation.to_owned()));
    true
}

#[cfg(test)]
mod tests;
