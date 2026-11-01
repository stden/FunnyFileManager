//! Purpose: The Language servers dialog's list and view commands, and the
//! one lookup from a key to an [`LspListCommand`] in the keys in force.
//!
//! The keys are the keymap's `language_servers` context, whose defaults are
//! `keymap::tables::LANGUAGE_SERVERS`. `popup::lsp_editor` applies the
//! command in the server list and in the F3 view; the form and the exit
//! question keep the dialog protocol. Resolving a key has no effect.

use crossterm::event::KeyEvent;

use super::Bindings;
use crate::models::lsp_editor::Step;

/// What one key asks of the server list, or of the F3 view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LspListCommand {
    /// Move the selection, or scroll the view.
    Move(Step),
    /// Open the selected server's form.
    Edit,
    /// Enable or disable the selected server.
    ToggleEnabled,
    /// Open the form for a new server.
    Add,
    /// Delete the selected server.
    Remove,
    /// Save the servers.
    Save,
    /// Show every server, scrolled; in the view, back to the list.
    View,
    /// Check every server's command again.
    Recheck,
    /// Close, asking first when something changed; in the view, back to
    /// the list.
    Close,
}

/// What `key` asks of the list or the view: the command `bindings` gives
/// the chord the key means.
///
/// The match is exact, so a modifier more makes another key: Shift+F8 is
/// not F8. `None` for a key nothing is bound to, which the dialog swallows.
#[must_use]
pub fn resolve(bindings: &Bindings<LspListCommand>, key: KeyEvent) -> Option<LspListCommand> {
    bindings.resolve(key)
}

#[cfg(test)]
mod tests;
