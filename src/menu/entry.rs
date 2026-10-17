//! Purpose: A drop-down row — its label, the key or command it runs, and the keystroke type.

use crossterm::event::{KeyCode, KeyModifiers};

use super::MenuCommand;

/// What a drop-down row is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Entry {
    /// An executable command with its label, shortcut hint, and checked state.
    Item {
        label: &'static str,
        shortcut: &'static str,
        activation: Activation,
        /// Drawn with Far's tick in the left margin.
        checked: bool,
    },
    /// A rule between groups.
    Separator,
}

/// Every menu item has exactly one executable route.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Activation {
    Shortcut(Shortcut),
    Command(MenuCommand),
}

impl Entry {
    pub(super) const fn item(label: &'static str, shortcut: &'static str, key: Shortcut) -> Self {
        Self::Item {
            label,
            shortcut,
            activation: Activation::Shortcut(key),
            checked: false,
        }
    }

    /// Whether the cursor can stop here.
    #[must_use]
    pub const fn is_selectable(&self) -> bool {
        matches!(self, Self::Item { .. })
    }

    pub(super) const fn command(
        label: &'static str,
        shortcut: &'static str,
        command: MenuCommand,
    ) -> Self {
        Self::Item {
            label,
            shortcut,
            activation: Activation::Command(command),
            checked: false,
        }
    }

    #[must_use]
    pub const fn menu_command(&self) -> Option<MenuCommand> {
        match self {
            Self::Item {
                activation: Activation::Command(command),
                ..
            } => Some(*command),
            _ => None,
        }
    }

    /// The key this row stands for.
    #[must_use]
    pub const fn key(&self) -> Option<Shortcut> {
        match self {
            Self::Item {
                activation: Activation::Shortcut(key),
                ..
            } => Some(*key),
            _ => None,
        }
    }
}

/// A keystroke a menu row stands for.
///
/// Stored rather than resolved to an [`crate::action::Action`]: the menu then
/// goes through the same dispatch the keyboard does, so a command cannot
/// behave differently depending on how it was reached.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Shortcut {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
}

impl Shortcut {
    pub(super) const fn new(code: KeyCode, modifiers: KeyModifiers) -> Self {
        Self { code, modifiers }
    }

    pub(super) const fn ctrl(ch: char) -> Self {
        Self::new(KeyCode::Char(ch), KeyModifiers::CONTROL)
    }

    pub(super) const fn plain(code: KeyCode) -> Self {
        Self::new(code, KeyModifiers::NONE)
    }

    pub(super) const fn alt(code: KeyCode) -> Self {
        Self::new(code, KeyModifiers::ALT)
    }

    pub(super) const fn shift(code: KeyCode) -> Self {
        Self::new(code, KeyModifiers::SHIFT)
    }
}
