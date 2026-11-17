//! Purpose: The PATH editor's browse commands, and the key that asks for
//! each: a lookup in the `path_editor` bindings of the keymap.
//!
//! `popup::path_env` applies the command while the list is browsed; the
//! entry field and the two questions keep the dialog protocol. The default
//! keys are the rows of `keymap::tables::PATH_EDITOR`. Resolving a key has
//! no effect.

use crossterm::event::KeyEvent;

use super::Bindings;

/// What one key asks of the PATH editor's list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathCommand {
    /// The cursor one row up.
    CursorUp,
    /// The cursor one row down.
    CursorDown,
    /// The cursor on the first entry.
    CursorHome,
    /// The cursor on the last entry.
    CursorEnd,
    /// The entry one row up.
    MoveEntryUp,
    /// The entry one row down.
    MoveEntryDown,
    /// A new entry.
    Add,
    /// The selected entry's text.
    Edit,
    /// Drop the selected entry.
    Remove,
    /// Check every entry again.
    Recheck,
    /// Save, after asking.
    Save,
    /// Close, asking first when something changed.
    Close,
}

/// What `key` asks of the list: the command `bindings` holds for exactly
/// that chord, a modifier more or less being another key. `None` for a key
/// nothing is bound to, which the list swallows.
#[must_use]
pub fn resolve(bindings: &Bindings<PathCommand>, key: KeyEvent) -> Option<PathCommand> {
    bindings.resolve(key)
}

#[cfg(test)]
mod tests;
