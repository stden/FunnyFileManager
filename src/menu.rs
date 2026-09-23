//! Purpose: Far's menu bar — the five headings, their drop-downs, and F9.
//!
//! The bar is always on screen, as Far's is. It is not only a way to reach
//! commands: it is where the program says what it can do, which for a file
//! manager with forty hotkeys is most of the documentation a user ever reads.
//!
//! Items route through shared keyboard dispatch or a typed menu command.
//! Every non-separator row has an executable route; operations live in ops.

use crossterm::event::{KeyCode, KeyModifiers};

mod command;
mod cursor;
mod tables;

pub use command::{MenuCommand, SettingsPage};
pub use cursor::Menu;

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
        Self::ALL[(self.position() + 1) % Self::ALL.len()]
    }

    #[must_use]
    pub fn previous(self) -> Self {
        Self::ALL[(self.position() + Self::ALL.len() - 1) % Self::ALL.len()]
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
    const fn item(label: &'static str, shortcut: &'static str, key: Shortcut) -> Self {
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

    const fn command(label: &'static str, shortcut: &'static str, command: MenuCommand) -> Self {
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
    const fn new(code: KeyCode, modifiers: KeyModifiers) -> Self {
        Self { code, modifiers }
    }

    const fn ctrl(ch: char) -> Self {
        Self::new(KeyCode::Char(ch), KeyModifiers::CONTROL)
    }

    const fn plain(code: KeyCode) -> Self {
        Self::new(code, KeyModifiers::NONE)
    }

    const fn alt(code: KeyCode) -> Self {
        Self::new(code, KeyModifiers::ALT)
    }

    const fn shift(code: KeyCode) -> Self {
        Self::new(code, KeyModifiers::SHIFT)
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
