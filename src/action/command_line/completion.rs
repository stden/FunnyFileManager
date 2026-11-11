//! Purpose: The keys of the command line's open completion list.
//!
//! One pure mapping from a key to a [`CompletionCommand`], applied by
//! `popup::command_line::keys`. A key the list does not use goes on to the
//! command line's own keys, so typing keeps editing the line under the list.

use crossterm::event::KeyEvent;

use crate::action::Bindings;

/// What one key asks of the open completion list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionCommand {
    /// Down, Tab: highlight the next candidate.
    Next,
    /// Up: highlight the previous candidate.
    Previous,
    /// Enter: splice the highlighted candidate into the line.
    Accept,
    /// Esc: close the list, leaving the line as it is.
    Close,
}

/// What `key` asks of the list: the command its chord is bound to in
/// `bindings`; `None` for a key the list hands on to the command line.
#[must_use]
pub fn resolve(bindings: &Bindings<CompletionCommand>, key: KeyEvent) -> Option<CompletionCommand> {
    bindings.resolve(key)
}

#[cfg(test)]
mod tests;
