//! Purpose: Running a [`Search`] — case folding, word boundaries and the wrapping walk.

use std::ops::Range;

use super::{Caret, Case, Search, WordMatch};

mod ascii;
mod exact;
mod unicode;

use exact::ExactFinders;

/// Characters that count as part of a word, for [`WordMatch::WholeWords`].
fn is_word_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

struct PreparedNeedle<'a> {
    raw: &'a str,
    lower_ascii: Vec<u8>,
    ascii_anchor: Option<(usize, u8)>,
    safe_ascii: Option<u8>,
    utf8_anchor: Option<unicode::Utf8Anchor>,
    has_non_ascii_folded: bool,
    exact: Option<ExactFinders<'a>>,
    chars: Vec<char>,
    scratch: Vec<char>,
}

impl<'a> PreparedNeedle<'a> {
    fn new(raw: &'a str, case: Case) -> Self {
        let lower_ascii: Vec<u8> = if raw.is_ascii() {
            raw.bytes().map(|b| b.to_ascii_lowercase()).collect()
        } else {
            Vec::new()
        };
        let ascii_anchor = ascii::rarest_anchor(&lower_ascii);
        let exact = (case == Case::Match).then(|| ExactFinders::new(raw));
        let mut chars = Vec::new();
        if case == Case::Ignore && !raw.is_ascii() {
            unicode::fold_into(raw, &mut chars);
        }
        let safe_ascii = (case == Case::Ignore)
            .then(|| unicode::safe_ascii_anchor(raw))
            .flatten();
        let utf8_anchor = (case == Case::Ignore && safe_ascii.is_none())
            .then(|| unicode::utf8_anchor(&chars))
            .flatten();
        let has_non_ascii_folded = chars.iter().any(|ch| !ch.is_ascii());
        Self {
            raw,
            lower_ascii,
            ascii_anchor,
            safe_ascii,
            utf8_anchor,
            has_non_ascii_folded,
            exact,
            chars,
            scratch: Vec::new(),
        }
    }
}

impl Search {
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
    #[must_use]
    pub fn find(&self, lines: &[String], from: Caret) -> Option<Caret> {
        if lines.is_empty() {
            return None;
        }
        let mut prepared = PreparedNeedle::new(self.needle.as_str(), self.case);
        let count = lines.len();
        let caret = from.column.chars();
        for step in 0..=count {
            let line = self.line_at_step(from.line.get(), step, count);
            if let Some(col) = lines
                .get(line)
                .and_then(|text| self.find_on_line(text, &mut prepared, step, count, caret))
            {
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
        if let Some(exact) = &prepared.exact {
            if !exact.contains_in(text.as_bytes()) {
                return None;
            }
            if prepared.raw.is_ascii() && text.is_ascii() {
                return self.find_in_ascii_line(text.as_bytes(), prepared, step, count, caret);
            }
            return self.find_exact_utf8(text, exact, step, count, caret);
        }
        if Self::ruled_out_ignore_case(text, prepared) {
            return None;
        }
        if prepared.raw.is_ascii() && text.is_ascii() {
            return self.find_in_ascii_line(text.as_bytes(), prepared, step, count, caret);
        }
        if prepared.chars.is_empty() {
            unicode::fold_into(prepared.raw, &mut prepared.chars);
        }
        unicode::fold_into(text, &mut prepared.scratch);
        self.find_in_folded(&prepared.scratch, &prepared.chars, step, count, caret)
    }

    fn ruled_out_ignore_case(text: &str, prepared: &PreparedNeedle<'_>) -> bool {
        let bytes = text.as_bytes();
        prepared
            .safe_ascii
            .is_some_and(|anchor| !unicode::has_ascii_anchor(bytes, anchor))
            || prepared
                .utf8_anchor
                .as_ref()
                .is_some_and(|anchor| !unicode::has_utf8_anchor(bytes, anchor))
            || (prepared.has_non_ascii_folded && text.is_ascii())
    }

    fn find_in_folded(
        &self,
        haystack: &[char],
        needle: &[char],
        step: usize,
        count: usize,
        caret: usize,
    ) -> Option<usize> {
        let last_start = haystack.len().checked_sub(needle.len())?;
        let range = self.column_range(step, count, caret, last_start);
        if self.direction.is_backward() {
            range
                .rev()
                .find(|&col| self.matches_at(haystack, needle, col))
        } else {
            let mut range = range;
            range.find(|&col| self.matches_at(haystack, needle, col))
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
