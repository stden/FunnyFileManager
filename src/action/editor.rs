//! Editor command DSL: describe intent without changing state or doing I/O.
//!
//! The keys are the keymap's `editor` context — the defaults of
//! `keymap::tables::EDITOR`, or what `ffm.yml` makes of them — and are
//! matched exactly: Shift+F2 is not F2. What is decided here is only what a
//! key means when nothing is bound to it.
//!
//! Confirmation and search prompts consume input before this resolver runs.
//! The interpreter in `popup::editor` owns state transitions and delegates
//! clipboard and persistence effects to operations.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::Bindings;
use crate::models::SelectionMode;

/// Navigation intent; the interpreter supplies the editor's page distance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorNavigation {
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    PageUp,
    PageDown,
}

/// One instruction in the editor's input language.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorCommand {
    Move {
        motion: EditorNavigation,
        selection: SelectionMode,
    },
    Copy,
    /// Ctrl+F, as in Far: type the edited file's full name at the cursor,
    /// and put it on the clipboard too.
    InsertFileName,
    SelectAll,
    ChooseEncoding,
    Paste,
    Save,
    Close,
    Search,
    RepeatSearch,
    FormatDocument,
    Lsp(crate::models::LspFeature),
    RestartLsp,
    Insert(char),
    Newline,
    Backspace,
    Delete,
    Indent,
    /// Unsupported input is consumed by the editor's modal boundary.
    Ignore,
}

/// The command `key` runs in the editor: the one its chord is bound to in
/// `bindings`, and for a key nothing is bound to, the editor's fallback.
#[must_use]
pub fn resolve(bindings: &Bindings<EditorCommand>, key: KeyEvent) -> EditorCommand {
    bindings.resolve(key).unwrap_or_else(|| fallback(key))
}

/// A key nothing is bound to. A character without Ctrl or Alt types itself,
/// read from the event as the terminal sent it, so `CapsLock`'s capital stays
/// a capital; every other key is consumed by the editor's modal boundary.
fn fallback(key: KeyEvent) -> EditorCommand {
    const COMMAND_MODIFIERS: KeyModifiers = KeyModifiers::CONTROL.union(KeyModifiers::ALT);
    match key.code {
        KeyCode::Char(ch) if !key.modifiers.intersects(COMMAND_MODIFIERS) => {
            EditorCommand::Insert(ch)
        }
        _ => EditorCommand::Ignore,
    }
}

#[cfg(test)]
mod tests;

pub(crate) fn selection_mode(modifiers: KeyModifiers) -> SelectionMode {
    if modifiers.contains(KeyModifiers::SHIFT) {
        SelectionMode::Extend
    } else {
        SelectionMode::Move
    }
}
