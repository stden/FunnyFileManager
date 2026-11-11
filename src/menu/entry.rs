//! Purpose: A drop-down row — its label, the shortcut it prints, and the action it runs.

use crate::action::Action;

/// What a drop-down row is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Entry {
    /// An executable command with its label, shortcut hint, and checked state.
    Item {
        label: &'static str,
        shortcut: &'static str,
        /// What the row runs: the same value its key resolves to, so a
        /// command cannot behave differently depending on how it was
        /// reached. A row Far gives no key holds an [`Action::Menu`].
        action: Action,
        /// Drawn with Far's tick in the left margin.
        checked: bool,
    },
    /// A rule between groups.
    Separator,
}

impl Entry {
    pub(super) const fn item(label: &'static str, shortcut: &'static str, action: Action) -> Self {
        Self::Item {
            label,
            shortcut,
            action,
            checked: false,
        }
    }

    /// Whether the cursor can stop here.
    #[must_use]
    pub const fn is_selectable(&self) -> bool {
        matches!(self, Self::Item { .. })
    }

    /// The action this row runs; a separator runs nothing.
    #[must_use]
    pub const fn action(&self) -> Option<Action> {
        match self {
            Self::Item { action, .. } => Some(*action),
            Self::Separator => None,
        }
    }
}
