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

/// Characters that count as part of a word, for [`WordMatch::WholeWords`].
fn is_word_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

impl Search {
    /// One line's characters, case-folded when the search ignores case.
    ///
    /// Folded one character at a time so the result stays index-for-index with
    /// the original — a whole-string `to_lowercase` can change the length (`İ`
    /// becomes two chars) and every column the search returned would then point
    /// somewhere else in the real line. The price is that a character whose
    /// lowercase is genuinely multi-character (`ß` → `ss`) folds to its first
    /// only, so `ß` does not match `SS`. That is the rarer wrong answer.
    fn fold(&self, text: &str) -> Vec<char> {
        match self.case {
            Case::Match => text.chars().collect(),
            Case::Ignore => text
                .chars()
                .map(|ch| ch.to_lowercase().next().unwrap_or(ch))
                .collect(),
        }
    }

    /// Whether `needle` sits at `at` in `haystack`, honouring whole-word mode.
    fn matches_at(&self, haystack: &[char], needle: &[char], at: usize) -> bool {
        let after = at + needle.len();
        if haystack.get(at..after) != Some(needle) {
            return false;
        }
        if self.words == WordMatch::Anywhere {
            return true;
        }
        // A word boundary is the *absence* of a word character, and the ends of
        // the line count as absent — otherwise a match against the first word
        // on a line could never be whole.
        let before = at.checked_sub(1).and_then(|index| haystack.get(index));
        let before_ok = before.is_none_or(|&ch| !is_word_char(ch));
        let after_ok = haystack.get(after).is_none_or(|&ch| !is_word_char(ch));
        before_ok && after_ok
    }

    /// The next match after `from`, wrapping once through the whole buffer.
    ///
    /// Wrapping rather than stopping at the end: Far asks whether to continue
    /// from the top, and answering "yes" is what the user wants every time —
    /// so the question is not worth a dialog. Every position is visited exactly
    /// once, so a search with no match terminates instead of circling.
    #[must_use]
    pub fn find(&self, lines: &[String], from: Caret) -> Option<Caret> {
        if lines.is_empty() {
            return None;
        }
        let needle = self.fold(self.needle.as_str());
        let count = lines.len();
        let caret = from.column.chars();

        for step in 0..=count {
            let line = if self.direction.is_backward() {
                // `+ count` keeps the subtraction inside `usize` when `step`
                // exceeds `from.line`.
                (from.line.get() + count - step % count) % count
            } else {
                (from.line.get() + step) % count
            };
            let Some(text) = lines.get(line) else {
                continue;
            };
            let haystack = self.fold(text);
            let Some(last_start) = haystack.len().checked_sub(needle.len()) else {
                continue;
            };

            if let Some(column) = self
                .columns(step, count, caret, last_start)
                .find(|&column| self.matches_at(&haystack, &needle, column))
            {
                return Some(Caret::new(line, column));
            }
        }
        None
    }

    /// Which columns of this line are still unvisited, in search order.
    ///
    /// The first and last steps are the same line — the one the caret is on —
    /// split at the caret so the search moves away from it and comes back to
    /// the near side only after everything else has been tried.
    fn columns(
        &self,
        step: usize,
        count: usize,
        caret: usize,
        last_start: usize,
    ) -> Box<dyn Iterator<Item = usize>> {
        // Exclusive ranges throughout, because the split at the caret has to be
        // able to produce *no* columns: with the caret in column 0 a backward
        // search has nothing before it, and `0..=caret - 1` cannot say so —
        // `0usize.saturating_sub(1)` is 0, which is the caret itself.
        let end = last_start + 1;
        let split = caret.min(end);
        if self.direction.is_backward() {
            let range = match step {
                // Strictly before the caret: a backward search must move, not
                // re-find the match the caret is already sitting on.
                0 => 0..split,
                s if s == count => split..end,
                _ => 0..end,
            };
            return Box::new(range.rev());
        }
        let after = (caret + 1).min(end);
        let range = match step {
            0 => after..end,
            s if s == count => 0..after,
            _ => 0..end,
        };
        Box::new(range)
    }
}

mod prompt;

pub use prompt::{SearchField, SearchPrompt, SearchState};

#[cfg(test)]
mod tests;
