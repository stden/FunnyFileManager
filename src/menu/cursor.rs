//! Purpose: the open menu's position — which heading and row carry the cursor — and how it moves.

use super::tables::{COMMANDS, FILES, OPTIONS, PANEL, PANEL_RIGHT};
use super::{hotkey_of, label_of, Entry, Heading};

/// Where the menu is: which heading, and which row of its drop-down.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Menu {
    pub heading: Heading,
    /// Which row of the drop-down carries the cursor.
    pub row: usize,
}

impl Menu {
    /// Open on `heading`, with the cursor on its first usable row.
    ///
    #[must_use]
    pub fn open(heading: Heading) -> Self {
        let mut menu = Self { heading, row: 0 };
        if !menu.entry().is_some_and(Entry::is_selectable) {
            menu.step(true);
        }
        menu
    }

    /// Whether any row of this drop-down can be chosen.
    #[must_use]
    pub fn has_anything_usable(&self) -> bool {
        self.entries().iter().any(Entry::is_selectable)
    }

    /// Whether the cursor is on a row that does something.
    #[must_use]
    pub fn is_on_a_usable_row(&self) -> bool {
        self.entry().is_some_and(Entry::is_selectable)
    }

    /// The drop-down under the current heading.
    #[must_use]
    pub const fn entries(&self) -> &'static [Entry] {
        match self.heading {
            Heading::Left => &PANEL,
            Heading::Right => &PANEL_RIGHT,
            Heading::Files => &FILES,
            Heading::Commands => COMMANDS,
            Heading::Options => &OPTIONS,
        }
    }

    /// The row under the cursor.
    #[must_use]
    pub fn entry(&self) -> Option<&'static Entry> {
        self.entries().get(self.row)
    }

    /// Move to another heading, landing on its first usable row.
    pub fn go(&mut self, forward: bool) {
        self.heading = if forward {
            self.heading.next()
        } else {
            self.heading.previous()
        };
        *self = Self::open(self.heading);
    }

    /// Move the cursor one usable row, stepping over separators.
    pub fn step(&mut self, down: bool) {
        let len = self.entries().len();
        if len == 0 {
            return;
        }
        for _ in 0..len {
            self.row = if down {
                (self.row + 1) % len
            } else {
                (self.row + len - 1) % len
            };
            if self.entry().is_some_and(Entry::is_selectable) {
                return;
            }
        }
    }

    /// The row whose hotkey letter is `ch`, if the open drop-down has one.
    #[must_use]
    pub fn row_for_hotkey(&self, ch: char) -> Option<usize> {
        let wanted = ch.to_ascii_lowercase();
        self.entries()
            .iter()
            .position(|entry| entry.is_selectable() && hotkey_of(label_of(entry)) == Some(wanted))
    }

    /// The heading whose hotkey letter is `ch`.
    #[must_use]
    pub fn heading_for_hotkey(ch: char) -> Option<Heading> {
        let wanted = ch.to_ascii_lowercase();
        Heading::ALL
            .into_iter()
            .find(|heading| hotkey_of(heading.label()) == Some(wanted))
    }
}
