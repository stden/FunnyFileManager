//! Purpose: The database explorer's commands and the lookup from a key to one.
//!
//! The keys are the keymap's `database_explorer` context, whose defaults are
//! `keymap::tables::DATABASE_EXPLORER`. `popup::database` applies the command;
//! resolving a key has no effect.

use crossterm::event::KeyEvent;

use super::Bindings;

/// What one key asks of the explorer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatabaseCommand {
    /// Move the cursor up, or scroll a rows or columns view.
    Up,
    Down,
    /// Expand the row under the cursor.
    Expand,
    /// Collapse the row under the cursor, or go to its parent.
    Collapse,
    /// Open a table's rows; a connection or schema toggles.
    Open,
    /// Show the columns of the table under the cursor.
    Columns,
    /// The previous page of rows.
    PageUp,
    /// The next page of rows.
    PageDown,
    /// Read the connection's metadata again.
    Reload,
    /// Back to the tree from a view; from the tree, close.
    Back,
    /// Close the explorer.
    Close,
}

/// What `key` asks of the explorer: the command `bindings` gives the chord the
/// key means. `None` for a key nothing is bound to, which the dialog swallows.
#[must_use]
pub fn resolve(bindings: &Bindings<DatabaseCommand>, key: KeyEvent) -> Option<DatabaseCommand> {
    bindings.resolve(key)
}

#[cfg(test)]
mod tests;
