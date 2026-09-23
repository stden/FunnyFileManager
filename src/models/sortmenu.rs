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

/// One row of the menu: a mode, or the order switch under the rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortRow {
    Mode(SortMode),
    /// Far's `-` and `*`: the same mode, run the other way.
    Reverse,
}

impl SortRow {
    /// In the order Far's help lists them, the order switch last.
    pub const ALL: [Self; 10] = [
        Self::Mode(SortMode::Name),
        Self::Mode(SortMode::Extension),
        Self::Mode(SortMode::ModificationTime),
        Self::Mode(SortMode::Size),
        Self::Mode(SortMode::Unsorted),
        Self::Mode(SortMode::CreationTime),
        Self::Mode(SortMode::AccessTime),
        Self::Mode(SortMode::Descriptions),
        Self::Mode(SortMode::Owner),
        Self::Reverse,
    ];

    /// The function key of the first mode row; the rest follow in menu order.
    const FIRST_KEY: usize = 3;

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Mode(mode) => mode.label(),
            Self::Reverse => "Reverse order",
        }
    }

    /// The key the row answers to outside the menu: `Ctrl+F3` … `Ctrl+F11`
    /// for the modes, and `*` — the menu's own order flip — for the switch.
    #[must_use]
    pub fn shortcut(self) -> String {
        match self {
            Self::Mode(mode) => {
                let index = SortMode::ALL.iter().position(|m| *m == mode);
                format!(
                    "Ctrl+F{}",
                    index.map_or(Self::FIRST_KEY, |i| i + Self::FIRST_KEY)
                )
            }
            Self::Reverse => "*".to_owned(),
        }
    }

    /// The letter that picks the row from the keyboard — the mode's panel
    /// header letter, so one letter means one mode everywhere.
    #[must_use]
    pub const fn hotkey(self) -> char {
        match self {
            Self::Mode(mode) => mode.letter(),
            Self::Reverse => 'r',
        }
    }

    /// Far's own mnemonic for the extension row — "e&Xtension" — kept as an
    /// alias, since a Far hand reaches for it before it reads the label.
    const FAR_EXTENSION_MNEMONIC: char = 'x';

    /// The row a typed letter names, in either case.
    #[must_use]
    pub fn by_hotkey(letter: char) -> Option<Self> {
        let wanted = letter.to_ascii_lowercase();
        if wanted == Self::FAR_EXTENSION_MNEMONIC {
            return Some(Self::Mode(SortMode::Extension));
        }
        Self::ALL.into_iter().find(|row| row.hotkey() == wanted)
    }

    /// The mode row `Ctrl+F<number>` names — the inverse of [`Self::shortcut`],
    /// so the key works inside the menu as it does outside it.
    #[must_use]
    pub fn by_function_key(number: u8) -> Option<Self> {
        let index = usize::from(number).checked_sub(Self::FIRST_KEY)?;
        SortMode::ALL.get(index).copied().map(Self::Mode)
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
