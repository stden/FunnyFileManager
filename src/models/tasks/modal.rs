//! Purpose: What is open inside the task list — a bar, a menu, a dialog, or
//! nothing — as one enum, so two can never be open at once, and the state
//! each one keeps: its text, its cursor, its focus and its frozen targets.
//!
//! The keys that move them live in `popup`; the invariants live here, in
//! constructors and private fields.

mod dialogs;

use crate::lineedit::LineEditor;
use crate::models::MaskPrompt;
use crate::scroll::ScrollTop;

use super::notice::Failure;
use super::targets::FrozenTargets;
use super::view::{Sort, SortKey, UserScope};
use super::{Owner, Pid, ProcessKey, Signal};

pub use dialogs::{ConfirmButton, FormFocus, KillConfirm, NiceConfirm, PriorityForm, ReniceButton};

/// What the task list shows on top of the table.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Modal {
    #[default]
    Browse,
    Help(ScrollTop),
    Info(ProcessKey),
    Filter(TextBar),
    Search(SearchBar),
    PidJump(PidQuery),
    Menu(TaskMenu),
    Mask(MaskPrompt),
    Confirm(KillConfirm),
    Priority(PriorityForm),
    /// `[` asks before lowering a priority a user cannot raise again.
    Renice(NiceConfirm),
    /// One line per target a delivery failed for.
    Error(Vec<Failure>),
}

/// The filter bar's line.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TextBar {
    line: LineEditor,
}

impl TextBar {
    /// A bar holding `text`, the caret at its end.
    #[must_use]
    pub fn new(text: &str) -> Self {
        Self {
            line: LineEditor::new(text),
        }
    }

    #[must_use]
    pub const fn line(&self) -> &LineEditor {
        &self.line
    }

    pub const fn line_mut(&mut self) -> &mut LineEditor {
        &mut self.line
    }
}

/// Whether the search found a row.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Found {
    #[default]
    Match,
    /// Drawn in the failed-prompt colour.
    NoMatch,
}

/// The search bar: its query and whether the last search found anything.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SearchBar {
    line: LineEditor,
    found: Found,
}

impl SearchBar {
    #[must_use]
    pub const fn line(&self) -> &LineEditor {
        &self.line
    }

    pub const fn line_mut(&mut self) -> &mut LineEditor {
        &mut self.line
    }

    #[must_use]
    pub const fn found(&self) -> Found {
        self.found
    }

    pub const fn set_found(&mut self, found: Found) {
        self.found = found;
    }
}

/// The digits typed for a PID jump: ASCII digits only, at most
/// [`PidQuery::MAX_DIGITS`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PidQuery(String);

impl PidQuery {
    /// Enough for any `u32`; every PID has fewer digits.
    pub const MAX_DIGITS: usize = 10;

    /// Appends `digit`; anything but an ASCII digit, or a digit past the
    /// limit, is ignored.
    pub fn push(&mut self, digit: char) {
        if digit.is_ascii_digit() && self.0.len() < Self::MAX_DIGITS {
            self.0.push(digit);
        }
    }

    /// Removes the last digit.
    pub fn pop(&mut self) {
        self.0.pop();
    }

    #[must_use]
    pub fn digits(&self) -> &str {
        &self.0
    }

    /// The PID the digits spell, or `None` for none, zero, or one above
    /// `i32::MAX`.
    #[must_use]
    pub fn pid(&self) -> Option<Pid> {
        self.0.parse::<u32>().ok().and_then(Pid::new)
    }
}

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
