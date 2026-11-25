//! Purpose: The keys of the command line's Ctrl+R reverse history search.
//!
//! The search owns the keyboard while it runs. One pure mapping from a key
//! to a [`LineSearchCommand`], applied by `popup::command_line::search`.

use crossterm::event::KeyEvent;

use super::typed;
use crate::action::Bindings;

/// What one key asks of the running search.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineSearchCommand {
    /// Ctrl+R again: the next older match.
    SearchOlder,
    /// Enter: keep the match on the line and return to editing.
    Accept,
    /// Esc, Ctrl+G (bash's abort), Ctrl+C: put the pre-search line back.
    Cancel,
    /// Backspace: shorten the query.
    Backspace,
    /// Any character without Ctrl or Alt extends the query.
    Extend(char),
}

/// What `key` asks of the search: the command its chord is bound to in
/// `bindings`, and for a key nothing is bound to, the character it adds to
/// the query; `None` for a key the search does not use.
#[must_use]
pub fn resolve(bindings: &Bindings<LineSearchCommand>, key: KeyEvent) -> Option<LineSearchCommand> {
    bindings
        .resolve(key)
        .or_else(|| typed(key).map(LineSearchCommand::Extend))
}

#[cfg(test)]
mod tests;
