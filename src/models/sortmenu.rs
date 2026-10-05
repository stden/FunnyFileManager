//! Purpose: The sort modes menu (`Ctrl+F12`): the modes as a list, the current one ticked.
//!
//! State only: the operations that fill it live in `ops`, and the keys that
//! move through it in `popup`.
//!
//! The menu carries a snapshot of the panel's mode and order rather than
//! reading the panel: every choice closes the menu, so the snapshot cannot go
//! stale, and the renderer — which is handed the menu alone — has what it
//! needs to draw the ticks.

use super::{PanelSide, SortMode};

mod row;

pub use row::SortRow;

/// Which way a sort mode runs.
///
/// `Panel::sort_reversed` is the flag the listing is built from; this is the
/// same fact where a name reads better than a bare `true` — the `+`, `-` and
/// `*` keys of Far's menu each name one of these transitions.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SortOrder {
    #[default]
    Direct,
    Reversed,
}

impl SortOrder {
    #[must_use]
    pub const fn from_reversed(reversed: bool) -> Self {
        if reversed {
            Self::Reversed
        } else {
            Self::Direct
        }
    }

    #[must_use]
    pub const fn is_reversed(self) -> bool {
        matches!(self, Self::Reversed)
    }

    #[must_use]
    pub const fn toggled(self) -> Self {
        match self {
            Self::Direct => Self::Reversed,
            Self::Reversed => Self::Direct,
        }
    }
}

/// The open menu: the cursor, and the panel's state as it was when it opened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SortMenu {
    pub cursor: usize,
    /// The mode that carries the tick.
    pub mode: SortMode,
    /// Whether the "Reverse order" row carries the tick.
    pub order: SortOrder,
    /// The panel the menu sorts — and, as in Far, the one it drops down over.
    pub side: PanelSide,
}

impl Default for SortMenu {
    fn default() -> Self {
        Self::open(SortMode::Name, SortOrder::Direct, PanelSide::Left)
    }
}

impl SortMenu {
    /// Open on the panel's current mode, as Far does, so Enter alone reverses.
    #[must_use]
    pub fn open(mode: SortMode, order: SortOrder, side: PanelSide) -> Self {
        let cursor = SortRow::ALL
            .iter()
            .position(|row| *row == SortRow::Mode(mode))
            .unwrap_or(0);
        Self {
            cursor,
            mode,
            order,
            side,
        }
    }

    /// The row under the cursor. A cursor past the end reads as the last row
    /// rather than panicking on a menu built by hand.
    #[must_use]
    pub fn row(&self) -> SortRow {
        let [.., last] = SortRow::ALL;
        SortRow::ALL.get(self.cursor).copied().unwrap_or(last)
    }

    #[must_use]
    pub fn is_ticked(&self, row: SortRow) -> bool {
        match row {
            SortRow::Mode(mode) => mode == self.mode,
            SortRow::Reverse => self.order.is_reversed(),
        }
    }

    pub fn up(&mut self) {
        let len = SortRow::ALL.len();
        self.cursor = (self.cursor.min(len - 1) + len - 1) % len;
    }

    pub fn down(&mut self) {
        self.cursor = (self.cursor + 1) % SortRow::ALL.len();
    }

    pub fn first(&mut self) {
        self.cursor = 0;
    }

    pub fn last(&mut self) {
        self.cursor = SortRow::ALL.len() - 1;
    }
}

#[cfg(test)]
mod tests;
