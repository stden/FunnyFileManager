//! Purpose: The tree panel's keys (`Ctrl+T`), one pure mapping from a key
//! to a [`TreeCommand`].
//!
//! `popup::tree` applies the command. A key that resolves to nothing falls
//! through to the file-panel actions, so `Ctrl+T` still turns the tree off,
//! `Tab` still switches panels and `:` still opens the command line.
//! Resolving a key has no effect.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::{Action, DialogCommand, Navigation, SelectionCommand};
use crate::models::ListMotion;

/// What one key asks of the tree panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreeCommand {
    /// Up, Down, `PgUp`, `PgDn`, Home, End.
    Move(ListMotion),
    /// Right: show the folder's subfolders, or step into the first.
    Expand,
    /// Left: hide the folder's subfolders, or climb to its parent.
    Collapse,
    /// Enter: make the panel a file list of the folder under the cursor.
    GoToFolder,
    /// `+`: Far's Gray +, the next folder at the same level.
    NextSibling,
    /// `-`: Far's Gray -, the previous folder at the same level.
    PreviousSibling,
    /// Ctrl+Enter: the next folder the search matches, wrapping.
    FindNext,
    /// Ctrl+R: read the tree again.
    RefreshTree,
    /// Esc: end the search.
    ClearSearch,
    /// Backspace: drop the last character of the search.
    ShortenSearch,
    /// Any other character without Ctrl: Far's fast find, without having
    /// to hold Alt — a tree row is nothing a typed command could run.
    Search(char),
    /// F5–F8: a file operation, which needs a file panel to act on.
    NeedsFilePanel,
}

impl TreeCommand {
    /// The tree's reading of a panels action that arrived without a key —
    /// a menu row, a mouse gesture — as [`resolve`] is of a key: the cursor
    /// moves, Open, Re-read and the group keys are the tree's, the file
    /// operations need a file panel, and `None` is the file panel's.
    #[must_use]
    pub const fn from_action(action: Action) -> Option<Self> {
        match action {
            Action::Navigate(Navigation::NavUp) => Some(Self::Move(ListMotion::Up)),
            Action::Navigate(Navigation::NavDown) => Some(Self::Move(ListMotion::Down)),
            Action::Navigate(Navigation::First) => Some(Self::Move(ListMotion::Home)),
            Action::Navigate(Navigation::Last) => Some(Self::Move(ListMotion::End)),
            Action::Navigate(Navigation::PageUp) => Some(Self::Move(ListMotion::PageUp)),
            Action::Navigate(Navigation::PageDown) => Some(Self::Move(ListMotion::PageDown)),
            Action::Navigate(Navigation::Open) => Some(Self::GoToFolder),
            Action::Refresh => Some(Self::RefreshTree),
            Action::Selection(SelectionCommand::SelectGroup) => Some(Self::NextSibling),
            Action::Selection(SelectionCommand::UnselectGroup) => Some(Self::PreviousSibling),
            Action::Copy
            | Action::Move
            | Action::DeleteConfirm
            | Action::Dialog(DialogCommand::Mkdir)
            | Action::Selection(SelectionCommand::InvertSelection) => Some(Self::NeedsFilePanel),
            _ => None,
        }
    }
}

/// What `key` asks of the tree; `None` for a key that is not the tree's.
#[must_use]
pub const fn resolve(key: KeyEvent) -> Option<TreeCommand> {
    use TreeCommand::{
        ClearSearch, Collapse, Expand, FindNext, GoToFolder, Move, NeedsFilePanel, NextSibling,
        PreviousSibling, RefreshTree, Search, ShortenSearch,
    };
    keymap! { key;
        KeyCode::Enter if has(CONTROL) => Some(FindNext),
        KeyCode::Char('r' | 'R') if has(CONTROL) => Some(RefreshTree),
        // Every other Ctrl chord is the file panel's or the program's.
        _ if has(CONTROL) => None,
        KeyCode::Enter => Some(GoToFolder),
        KeyCode::Esc => Some(ClearSearch),
        KeyCode::Backspace => Some(ShortenSearch),
        KeyCode::F(5..=8) => Some(NeedsFilePanel),
        // The command line's opener.
        KeyCode::Char(':') => None,
        // Terminals cannot tell the pad's keys from the row's, so the plain
        // characters serve, as they do for marking on a file panel.
        KeyCode::Char('+') => Some(NextSibling),
        KeyCode::Char('-') => Some(PreviousSibling),
        KeyCode::Char(ch) => Some(Search(ch)),
        KeyCode::Right => Some(Expand),
        KeyCode::Left => Some(Collapse),
        KeyCode::Up => Some(Move(ListMotion::Up)),
        KeyCode::Down => Some(Move(ListMotion::Down)),
        KeyCode::Home => Some(Move(ListMotion::Home)),
        KeyCode::End => Some(Move(ListMotion::End)),
        KeyCode::PageUp => Some(Move(ListMotion::PageUp)),
        KeyCode::PageDown => Some(Move(ListMotion::PageDown)),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
