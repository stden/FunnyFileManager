//! Purpose: Far's menu bar — the five headings, their drop-downs, and F9.
//!
//! The bar is always on screen, as Far's is. It is not only a way to reach
//! commands: it is where the program says what it can do, which for a file
//! manager with forty hotkeys is most of the documentation a user ever reads.
//!
//! Items route through shared keyboard dispatch or a typed menu command.
//! Every non-separator row has an executable route; operations live in ops.

mod command;
mod cursor;
mod entry;
mod tables;

pub use command::{MenuCommand, SettingsPage};
pub use cursor::Menu;
pub use entry::{Activation, Entry, Shortcut};

/// One heading on the bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Heading {
    /// The left panel's own settings.
    #[default]
    Left,
    /// What can be done to the files under the cursor.
    Files,
    /// Everything that is not about one file.
    Commands,
    Options,
    /// The right panel's settings — the same list as `Left`.
    Right,
}

impl Heading {
    /// Left to right, as Far draws them.
    pub const ALL: [Self; 5] = [
        Self::Left,
        Self::Files,
        Self::Commands,
        Self::Options,
        Self::Right,
    ];

    /// The label, carrying the marker for its hotkey letter.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Left => "&Left",
            Self::Files => "&Files",
            Self::Commands => "&Commands",
            Self::Options => "&Options",
            Self::Right => "&Right",
        }
    }

    fn position(self) -> usize {
        Self::ALL
            .iter()
            .position(|heading| *heading == self)
            .unwrap_or(0)
    }

    /// The next heading, wrapping — a menu bar is a ring.
    #[must_use]
    pub fn next(self) -> Self {
        self.round(1)
    }

    #[must_use]
    pub fn previous(self) -> Self {
        self.round(Self::ALL.len() - 1)
    }

    /// The heading `steps` places to the right, wrapping. The index is taken
    /// modulo the ring's length, so the fallback is never used.
    fn round(self, steps: usize) -> Self {
        let index = (self.position() + steps) % Self::ALL.len();
        Self::ALL.get(index).copied().unwrap_or(self)
    }

    /// The panel this heading configures, for the two that name one.
    #[must_use]
    pub const fn side(self) -> Option<crate::models::PanelSide> {
        match self {
            Self::Left => Some(crate::models::PanelSide::Left),
            Self::Right => Some(crate::models::PanelSide::Right),
            _ => None,
        }
    }
}

/// An entry's label, marker included.
#[must_use]
pub const fn label_of(entry: &Entry) -> &'static str {
    match entry {
        Entry::Item { label, .. } => label,
        Entry::Separator => "",
    }
}

/// The letter a label's `&` marks, lowercased.
fn hotkey_of(label: &str) -> Option<char> {
    label
        .split_once('&')
        .and_then(|(_, rest)| rest.chars().next())
        .map(|ch| ch.to_ascii_lowercase())
}

#[cfg(test)]
mod tests;
