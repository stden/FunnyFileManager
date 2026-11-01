//! Purpose: The lines the task list edits in place: the filter bar, the
//! search bar with whether it found a row, and the digits of a PID jump.

use crate::lineedit::LineEditor;

use super::super::Pid;

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
