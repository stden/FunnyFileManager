//! Purpose: The menus open over the task list — Sort by, Send signal and Show
//! processes of — each with a cursor that is always on one of its rows.

use super::super::targets::FrozenTargets;
use super::super::view::{Sort, SortKey, UserScope};
use super::super::{Owner, Signal};

/// A menu row under the cursor: always less than the menu's length.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MenuCursor(usize);

impl MenuCursor {
    /// Row `index` of a menu `len` rows long, clamped to the last row;
    /// `None` for an empty menu, which has no cursor.
    #[must_use]
    pub fn at(index: usize, len: usize) -> Option<Self> {
        let last = len.checked_sub(1)?;
        Some(Self(index.min(last)))
    }

    #[must_use]
    pub const fn index(self) -> usize {
        self.0
    }
}

/// A menu open over the table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskMenu {
    /// Sort by: one row per [`SortKey`], then Reverse order.
    Sort(MenuCursor),
    /// Send signal: one row per offered signal, for targets frozen at `F9`.
    Signal {
        targets: FrozenTargets,
        cursor: MenuCursor,
    },
    /// Show processes of: All users, then each owner, frozen at `u`.
    User {
        owners: Vec<Owner>,
        cursor: MenuCursor,
    },
}

impl TaskMenu {
    /// Rows of the Sort by menu: every key and Reverse order.
    pub const SORT_ROWS: usize = SortKey::ALL.len() + 1;

    /// The Sort by menu, on the key in force.
    #[must_use]
    pub fn sort(current: Sort) -> Self {
        let index = SortKey::ALL
            .iter()
            .position(|key| *key == current.key())
            .unwrap_or(0);
        Self::Sort(MenuCursor(index))
    }

    /// The Send signal menu on SIGTERM, or `None` where no signal is offered
    /// and `F9` does what `F8` does.
    #[must_use]
    pub fn signal(targets: FrozenTargets) -> Option<Self> {
        let cursor = MenuCursor::at(0, Signal::OFFERED.len())?;
        Some(Self::Signal { targets, cursor })
    }

    /// The Show processes of menu over `owners`, on the scope in force.
    #[must_use]
    pub fn user(owners: Vec<Owner>, scope: &UserScope) -> Self {
        let index = match scope {
            UserScope::All => 0,
            UserScope::Only(owner) => owners
                .iter()
                .position(|listed| listed == owner)
                .map_or(0, |at| at + 1),
        };
        Self::User {
            cursor: MenuCursor(index.min(owners.len())),
            owners,
        }
    }

    /// Rows in the menu.
    #[must_use]
    pub fn row_count(&self) -> usize {
        match self {
            Self::Sort(_) => Self::SORT_ROWS,
            Self::Signal { .. } => Signal::OFFERED.len(),
            Self::User { owners, .. } => owners.len().saturating_add(1),
        }
    }

    #[must_use]
    pub const fn cursor(&self) -> MenuCursor {
        match self {
            Self::Sort(cursor) | Self::Signal { cursor, .. } | Self::User { cursor, .. } => *cursor,
        }
    }

    /// Moves the cursor to row `index`, clamped to the menu.
    pub fn select(&mut self, index: usize) {
        let Some(moved) = MenuCursor::at(index, self.row_count()) else {
            return;
        };
        match self {
            Self::Sort(cursor) | Self::Signal { cursor, .. } | Self::User { cursor, .. } => {
                *cursor = moved;
            }
        }
    }
}
