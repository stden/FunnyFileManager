//! Purpose: Running a [`Search`] — case folding, word boundaries and the wrapping walk.

use std::ops::Range;

use super::{Caret, Case, Search, WordMatch};

mod ascii;

/// Characters that count as part of a word, for [`WordMatch::WholeWords`].
fn is_word_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

struct PreparedNeedle<'a> {
    raw: &'a str,
    lower_ascii: Vec<u8>,
    chars: Option<Vec<char>>,
}

impl<'a> PreparedNeedle<'a> {
    fn new(raw: &'a str) -> Self {
        let lower_ascii = if raw.is_ascii() {
            raw.bytes().map(|b| b.to_ascii_lowercase()).collect()
        } else {
            Vec::new()
        };
        Self {
            raw,
            lower_ascii,
            chars: None,
        }
    }
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
    pub(super) fn matches_at(&self, haystack: &[char], needle: &[char], at: usize) -> bool {
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
        let mut prepared = PreparedNeedle::new(self.needle.as_str());
        let count = lines.len();
        let caret = from.column.chars();
        for step in 0..=count {
            let line = self.line_at_step(from.line.get(), step, count);
            let Some(text) = lines.get(line) else {
                continue;
            };
            if let Some(col) = self.find_on_line(text, &mut prepared, step, count, caret) {
                return Some(Caret::new(line, col));
            }
        }
        None
    }

    const fn line_at_step(&self, start_line: usize, step: usize, count: usize) -> usize {
        if self.direction.is_backward() {
            (start_line + count - step % count) % count
        } else {
            (start_line + step) % count
        }
    }

    fn find_on_line(
        &self,
        text: &str,
        prepared: &mut PreparedNeedle<'_>,
        step: usize,
        count: usize,
        caret: usize,
    ) -> Option<usize> {
        if prepared.raw.is_ascii() && text.is_ascii() {
            return self.find_in_ascii_line(
                text.as_bytes(),
                prepared.raw.as_bytes(),
                &prepared.lower_ascii,
                step,
                count,
                caret,
            );
        }
        if self.case == Case::Match
            && memchr::memmem::find(text.as_bytes(), prepared.raw.as_bytes()).is_none()
        {
            return None;
        }
        let needle = prepared
            .chars
            .get_or_insert_with(|| self.fold(prepared.raw));
        self.find_in_unicode_line(text, needle, step, count, caret)
    }

    fn find_in_unicode_line(
        &self,
        text: &str,
        needle: &[char],
        step: usize,
        count: usize,
        caret: usize,
    ) -> Option<usize> {
        let haystack = self.fold(text);
        let last_start = haystack.len().checked_sub(needle.len())?;
        let range = self.column_range(step, count, caret, last_start);
        if self.direction.is_backward() {
            range
                .rev()
                .find(|&col| self.matches_at(&haystack, needle, col))
        } else {
            let mut range = range;
            range.find(|&col| self.matches_at(&haystack, needle, col))
        }
    }

    /// Which columns of this line are still unvisited, as a half-open range.
    pub(super) fn column_range(
        &self,
        step: usize,
        count: usize,
        caret: usize,
        last_start: usize,
    ) -> Range<usize> {
        let end = last_start + 1;
        let split = caret.min(end);
        if self.direction.is_backward() {
            return match step {
                0 => 0..split,
                s if s == count => split..end,
                _ => 0..end,
            };
        }
        let after = (caret + 1).min(end);
        match step {
            0 => after..end,
            s if s == count => 0..after,
            _ => 0..end,
        }
    }
}
