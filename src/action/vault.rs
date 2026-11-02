//! Purpose: The `KeePass` vault browser's keys, one pure mapping from a key
//! to a [`VaultCommand`] through the `vault` bindings of the keymap.
//!
//! `popup::kdbx` applies the command while the vault is browsed, locked or
//! unlocked, and takes `Save` and `Undo` from it in the entry form; the
//! form's other keys and the vault's two dialogs keep the dialog protocol.
//! Every key is the vault's: a key that resolves to nothing is swallowed.
//! Resolving a key has no effect.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::Bindings;
use crate::models::ListMotion;

/// What one key asks of the vault.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VaultCommand {
    /// Up, Down, `PgUp`, `PgDn`, Home, End.
    Move(ListMotion),
    /// Enter: unlock while locked; copy the password once unlocked.
    Activate,
    /// Ctrl+U: copy the user name.
    CopyUsername,
    /// Ctrl+R: show the master password while locked; the entry's
    /// password dialog once unlocked.
    Reveal,
    /// Ctrl+Z: undo the last change, in the list or in the entry form.
    Undo,
    /// F4: the entry form.
    Edit,
    /// F6: the entries as a list or as a tree.
    ToggleView,
    /// F8: delete the entry, after asking.
    Delete,
    /// Ctrl+L: forget every key and close.
    Lock,
    /// Ctrl+C: lock, close, and quit ffm.
    LockAndQuit,
    /// Backspace: shorten the password or the filter.
    Backspace,
    /// Esc: remember the session and close.
    Close,
    /// F2: save the entry form.
    Save,
    /// Any character without Ctrl or Alt: typed into the password while
    /// locked, into the filter once unlocked.
    Type(char),
}

/// What `key` asks of the vault under `bindings`: the command its chord is
/// bound to, exactly (Shift+F8 is not F8), or else the character it types.
/// `None` for a key the vault does not use.
#[must_use]
pub fn resolve(bindings: &Bindings<VaultCommand>, key: KeyEvent) -> Option<VaultCommand> {
    bindings.resolve(key).or_else(|| typed(key))
}

/// The vault's fallback: a character without Ctrl or Alt is typed as the
/// terminal sent it, so a password keeps its case under `CapsLock`.
const fn typed(key: KeyEvent) -> Option<VaultCommand> {
    let modified = key
        .modifiers
        .intersects(KeyModifiers::CONTROL.union(KeyModifiers::ALT));
    match key.code {
        KeyCode::Char(ch) if !modified => Some(VaultCommand::Type(ch)),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
