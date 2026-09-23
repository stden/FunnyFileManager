//! Editor command DSL: describe intent without changing state or doing I/O.
//!
//! Confirmation and search prompts consume input before this resolver runs.
//! The interpreter in `popup::editor` owns state transitions and delegates
//! clipboard and persistence effects to operations.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

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
    SelectAll,
    ChooseEncoding,
    Paste,
    Save,
    Close,
    Search,
    RepeatSearch,
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

/// Resolve a key into a typed command using the shared ordered binding DSL.
#[must_use]
pub fn resolve(key: KeyEvent) -> EditorCommand {
    resolve_lsp(key).unwrap_or_else(|| resolve_editing(key))
}

fn resolve_lsp(key: KeyEvent) -> Option<EditorCommand> {
    use crate::models::LspFeature;
    keymap! { key;
        KeyCode::Char(' ') if exact(CONTROL) => Some(EditorCommand::Lsp(LspFeature::Completion)),
        KeyCode::Char('k' | 'K') if exact(CONTROL) => Some(EditorCommand::Lsp(LspFeature::Hover)),
        KeyCode::Char(']') if exact(CONTROL) => Some(EditorCommand::Lsp(LspFeature::Definition)),
        KeyCode::Char('r' | 'R') if exact(CONTROL + ALT) => Some(EditorCommand::RestartLsp),
        _ => None,
    }
}

fn resolve_editing(key: KeyEvent) -> EditorCommand {
    use EditorCommand::{Backspace, Close, Copy, Delete, Ignore, Indent, Insert, Move};
    use EditorCommand::{Newline, Paste, RepeatSearch, Save, Search};
    use EditorNavigation as Nav;

    let selection = selection_mode(key.modifiers);

    keymap! { key;
        KeyCode::Char('a' | 'A') if exact(CONTROL) => EditorCommand::SelectAll,
        KeyCode::Insert | KeyCode::Char('c' | 'C') if exact(CONTROL) => Copy,
        KeyCode::Char('v' | 'V') if exact(CONTROL) => Paste,
        KeyCode::Insert if exact(SHIFT) => Paste,
        KeyCode::Left => Move { motion: Nav::Left, selection },
        KeyCode::Right => Move { motion: Nav::Right, selection },
        KeyCode::Up => Move { motion: Nav::Up, selection },
        KeyCode::Down => Move { motion: Nav::Down, selection },
        KeyCode::Home => Move { motion: Nav::Home, selection },
        KeyCode::End => Move { motion: Nav::End, selection },
        KeyCode::PageUp => Move { motion: Nav::PageUp, selection },
        KeyCode::PageDown => Move { motion: Nav::PageDown, selection },
        KeyCode::F(7) if has(SHIFT) => RepeatSearch,
        KeyCode::F(7) => Search,
        KeyCode::F(2) => Save,
        KeyCode::F(8) => EditorCommand::ChooseEncoding,
        KeyCode::Esc | KeyCode::F(10) => Close,
        KeyCode::Enter => Newline,
        KeyCode::Backspace => Backspace,
        KeyCode::Delete => Delete,
        KeyCode::Tab => Indent,
        KeyCode::Char(ch) if without(CONTROL + ALT) => Insert(ch),
        _ => Ignore,
    }
}

#[cfg(test)]
mod tests;

fn selection_mode(modifiers: KeyModifiers) -> SelectionMode {
    if modifiers.contains(KeyModifiers::SHIFT) {
        SelectionMode::Extend
    } else {
        SelectionMode::Move
    }
}
