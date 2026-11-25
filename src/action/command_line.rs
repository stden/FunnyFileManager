//! Purpose: The `:` command line's keys, one pure mapping per mode.
//!
//! Editing resolves here to a [`LineCommand`]; the Ctrl+R history search
//! ([`search`]) and the open completion list ([`completion`]) have key maps
//! of their own, because the same key means something else while they are
//! up. `popup::command_line` picks the mode and applies the command;
//! resolving a key has no effect.
//!
//! The keys are the keymap's `command_line`, `command_line_search` and
//! `command_line_completion` contexts — the defaults of `keymap::tables`,
//! which the comments here name, or what `ffm.yml` makes of them — and are
//! matched exactly: Shift+Enter is not Enter. What is decided here is only
//! what a key means when nothing is bound to it.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::Bindings;

pub mod completion;
pub mod search;

/// What one key asks of the command line while it is being edited.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineCommand {
    /// Up: the previous history entry.
    HistoryBack,
    /// Down: the next history entry, or the line being typed.
    HistoryForward,
    /// Ctrl+R: start the reverse history search.
    HistorySearch,
    /// Tab: complete the word under the caret.
    Complete,
    /// Enter: run the line.
    Submit,
    /// Esc: close the command line, discarding the line.
    Cancel,
    /// Ctrl+C: copies a selection, as in any field; with nothing selected it
    /// is bash's "abandon this line" — clear it, and stay open.
    CopyOrClear,
    /// Ctrl+D: bash's EOF on an empty line closes; otherwise a forward
    /// delete, like every readline.
    DeleteCharOrClose,
    /// A move of the caret.
    Move(LineMotion),
    /// A change to the text.
    Edit(LineEdit),
    /// A caret move that extends the selection: Shift with the move, and
    /// Ctrl+Shift by word.
    Select(LineMotion),
    /// A clipboard operation on the selection.
    Clipboard(LineClipboard),
}

/// A move of the caret.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineMotion {
    /// Left.
    CharLeft,
    /// Right.
    CharRight,
    /// Ctrl+Left, Alt+Left, Alt+B.
    WordLeft,
    /// Ctrl+Right, Alt+Right, Alt+F.
    WordRight,
    /// Home, Ctrl+A.
    LineStart,
    /// End, Ctrl+E.
    LineEnd,
}

/// A change to the text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineEdit {
    /// Ctrl+W.
    KillWordLeft,
    /// Ctrl+U.
    KillToStart,
    /// Ctrl+K.
    KillToEnd,
    /// Ctrl+Y.
    Yank,
    /// Backspace.
    Backspace,
    /// Del.
    DeleteChar,
    /// Any character without Ctrl or Alt types itself.
    Insert(char),
}

/// A clipboard operation on the selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineClipboard {
    /// Ctrl+Insert.
    Copy,
    /// Ctrl+X and Shift+Del.
    Cut,
    /// Ctrl+V and Shift+Insert.
    Paste,
}

/// What `key` asks of the command line: the command its chord is bound to
/// in `bindings`, and for a key nothing is bound to, the character it
/// types; `None` for a key the command line does not use.
#[must_use]
pub fn resolve(bindings: &Bindings<LineCommand>, key: KeyEvent) -> Option<LineCommand> {
    bindings
        .resolve(key)
        .or_else(|| typed(key).map(|ch| LineCommand::Edit(LineEdit::Insert(ch))))
}

/// The character `key` types: one pressed without Ctrl or Alt, read from
/// the event as the terminal sent it, so `CapsLock`'s capital stays a
/// capital. The fallback of the line and of its history search.
fn typed(key: KeyEvent) -> Option<char> {
    const COMMAND_MODIFIERS: KeyModifiers = KeyModifiers::CONTROL.union(KeyModifiers::ALT);
    match key.code {
        KeyCode::Char(ch) if !key.modifiers.intersects(COMMAND_MODIFIERS) => Some(ch),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
