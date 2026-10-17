//! WSL dialog key language. Resolving input never starts a process.
//!
//! The keys are the `wsl_manager` context of the keymap: [`resolve`] looks
//! the key's chord up in the bindings it is handed and names no key itself.

use crossterm::event::KeyEvent;

use super::Bindings;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WslCommand {
    Up,
    Down,
    First,
    Last,
    Open,
    Refresh,
    Update,
    UpdatePackages,
    Close,
    /// The answer to a key nothing is bound to: the dialog consumes it.
    Ignore,
}

/// The command `key` runs in the dialog, as `bindings` gives it.
///
/// Only the chord exactly as bound runs its command, so Shift+Enter is not
/// Enter and Ctrl+Up is not Up. Every other key is [`WslCommand::Ignore`],
/// which keeps it inside the dialog.
#[must_use]
pub fn resolve(bindings: &Bindings<WslCommand>, key: KeyEvent) -> WslCommand {
    bindings.resolve(key).unwrap_or(WslCommand::Ignore)
}

#[cfg(test)]
mod tests;
