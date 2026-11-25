//! Purpose: The editor's F7 text search — what to look for, and where it is.
//!
//! Kept apart from both the editor and the screen so the hard part is testable
//! on its own: "find the next match" is arithmetic over char positions, and
//! every bug it can have — an off-by-one at the caret, a wrap that revisits or
//! skips a line, a word boundary at the end of a line — is visible without a
//! terminal.
//!
//! The options are named types rather than three `bool` fields. `find(&lines,
//! caret, true, false, true)` is unreadable at the call site and, worse, its
//! arguments are interchangeable: swapping two of them compiles and silently
//! searches for something else.

use crate::models::CharColumn;

/// Which way through the buffer a search walks.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Direction {
    #[default]
    Forward,
    Backward,
}

impl Direction {
    #[must_use]
    pub const fn is_backward(self) -> bool {
        matches!(self, Self::Backward)
    }
}

/// Whether case is part of the comparison.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Case {
    /// `Hello` matches `hello` — what a reader looking for a word wants.
    #[default]
    Ignore,
    Match,
}

/// Whether a match has to stand alone.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum WordMatch {
    /// Anywhere inside a longer word: `cat` matches `concatenate`.
    #[default]
    Anywhere,
    WholeWords,
}

/// The text being searched for.
///
/// A newtype because "search for nothing" has no answer, and the alternative is
/// every caller remembering to check — the same reasoning that made
/// `ShellCommand` reject an empty command. `Needle::new` is the only way to
/// build one, so a `Search` that exists is a search that can be run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Needle(String);

impl Needle {
    /// The needle for `text`, or `None` when there is nothing to look for.
    #[must_use]
    pub fn new(text: &str) -> Option<Self> {
        (!text.is_empty()).then(|| Self(text.to_owned()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A buffer line, counted from zero.
///
/// Distinct from [`CharColumn`] so the two halves of a [`Caret`] cannot be
/// passed in the wrong order — they are both `usize` underneath, and that is
/// exactly the mistake a compiler should be catching.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct LineIndex(usize);

impl LineIndex {
    #[must_use]
    pub const fn new(index: usize) -> Self {
        Self(index)
    }

    #[must_use]
    pub const fn get(self) -> usize {
        self.0
    }
}

/// A position in the buffer, in the units the editor moves the caret in.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Caret {
    pub line: LineIndex,
    /// Characters, never bytes — that is what `Editor::cursor_col` is, and
    /// converting at the boundary means the search never has to know how the
    /// lines are encoded.
    pub column: CharColumn,
}

impl Caret {
    #[must_use]
    pub const fn new(line: usize, column: usize) -> Self {
        Self {
            line: LineIndex::new(line),
            column: CharColumn::new(column),
        }
    }
}

/// A search the user has described in the F7 dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Search {
    pub needle: Needle,
    pub case: Case,
    pub words: WordMatch,
    pub direction: Direction,
}

impl Search {
    /// A plain search for `needle`: forwards, ignoring case, matching anywhere.
    #[must_use]
    pub fn new(needle: Needle) -> Self {
        Self {
            needle,
            case: Case::default(),
            words: WordMatch::default(),
            direction: Direction::default(),
        }
    }
}

mod matching;
mod prompt;

pub use prompt::{SearchField, SearchPrompt, SearchState};

#[cfg(test)]
mod tests;
