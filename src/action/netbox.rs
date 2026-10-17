//! Purpose: The commands of the two `NetBox` panels, the stored-sessions
//! list and a remote host, and the key that asks for each: the
//! `netbox_panel` keys of the keymap, matched exactly.
//!
//! Both panels are drawn over the file list, so the keys they own are few:
//! the list moves, Open, and the commands Far's `NetBox` gives a session.
//! Which key runs which is the keymap's (`keymap::tables::NETBOX_PANEL` by
//! default, `ffm.yml` over it); `popup::sessions` and `popup::remote` apply
//! the command, and a key that resolves to nothing is theirs to refuse or
//! hand back. Resolving a key has no effect.

use crossterm::event::KeyEvent;

use super::{Action, Bindings, DialogCommand, Navigation};
use crate::models::ListMotion;

/// What one key asks of a `NetBox` panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetboxCommand {
    /// A move through the rows; Up, Down, `PgUp`, `PgDn`, Home and End by
    /// default.
    Move(ListMotion),
    /// A new session, wherever the cursor is; Shift+F4 by default.
    NewSession,
    /// A command on the row under the cursor.
    Row(RowCommand),
}

/// A command on the row under the cursor, beside its default key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowCommand {
    /// Enter: connect to the session or enter the folder, or leave on `..`.
    Open,
    /// F3: see the session.
    ViewSession,
    /// F4: change the session.
    EditSession,
    /// F5: copy the session under a new name.
    CopySession,
    /// F8: delete the session, after asking.
    DeleteSession,
}

impl NetboxCommand {
    /// The panel's reading of a panels action that arrived without a key —
    /// a menu row, a mouse gesture — as [`resolve`] is of a key: the cursor
    /// moves, Open, and the file actions whose keys Far's `NetBox` gives a
    /// session. `None` is refused or handed back, as an unowned key is.
    #[must_use]
    pub const fn from_action(action: Action) -> Option<Self> {
        match action {
            Action::Navigate(Navigation::NavUp) => Some(Self::Move(ListMotion::Up)),
            Action::Navigate(Navigation::NavDown) => Some(Self::Move(ListMotion::Down)),
            Action::Navigate(Navigation::First) => Some(Self::Move(ListMotion::Home)),
            Action::Navigate(Navigation::Last) => Some(Self::Move(ListMotion::End)),
            Action::Navigate(Navigation::PageUp) => Some(Self::Move(ListMotion::PageUp)),
            Action::Navigate(Navigation::PageDown) => Some(Self::Move(ListMotion::PageDown)),
            Action::Navigate(Navigation::Open) => Some(Self::Row(RowCommand::Open)),
            Action::View => Some(Self::Row(RowCommand::ViewSession)),
            Action::Edit => Some(Self::Row(RowCommand::EditSession)),
            Action::Copy => Some(Self::Row(RowCommand::CopySession)),
            Action::DeleteConfirm => Some(Self::Row(RowCommand::DeleteSession)),
            Action::Dialog(DialogCommand::NewFile) => Some(Self::NewSession),
            _ => None,
        }
    }
}

/// What `key` asks of the panel with the keys in force, `bindings` being
/// the keymap's `netbox_panel`; `None` for a key the panel does not own.
/// The match is exact: Shift+Up is not Up.
#[must_use]
pub fn resolve(bindings: &Bindings<NetboxCommand>, key: KeyEvent) -> Option<NetboxCommand> {
    bindings.resolve(key)
}

#[cfg(test)]
mod tests;
