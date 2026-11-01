//! Purpose: Zero-allocation ASCII and SIMD fast paths for editor text search.

use std::ops::Range;

use super::super::{Case, Search, WordMatch};

fn is_ascii_word_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

fn ascii_word_ok(haystack: &[u8], at: usize, len: usize) -> bool {
    let before_ok = at
        .checked_sub(1)
        .and_then(|prev| haystack.get(prev))
        .is_none_or(|&byte| !is_ascii_word_byte(byte));
    let after_ok = haystack
        .get(at + len)
        .is_none_or(|&byte| !is_ascii_word_byte(byte));
    before_ok && after_ok
}

fn ascii_ignore_case_at(
    bytes: &[u8],
    needle_lower: &[u8],
    first: u8,
    last: u8,
    col: usize,
) -> bool {
    let len = needle_lower.len();
    bytes
        .get(col)
        .is_some_and(|b| b.to_ascii_lowercase() == first)
        && bytes
            .get(col + len - 1)
            .is_some_and(|b| b.to_ascii_lowercase() == last)
        && bytes
            .get(col..col + len)
            .is_some_and(|slice| slice.eq_ignore_ascii_case(needle_lower))
}

impl Search {
    /// Fast path for ASCII needles over ASCII lines without per-line heap allocations.
    pub(super) fn find_in_ascii_line(
        &self,
        bytes: &[u8],
        needle_raw: &[u8],
        needle_lower: &[u8],
        step: usize,
        count: usize,
        caret: usize,
    ) -> Option<usize> {
        let len = needle_raw.len();
        let last_start = bytes.len().checked_sub(len)?;
        let range = self.column_range(step, count, caret, last_start);
        if range.is_empty() {
            return None;
        }
        if self.case == Case::Match {
            return self.find_ascii_exact(bytes, needle_raw, range);
        }
        self.find_ascii_ignore_case(bytes, needle_lower, range)
    }

    fn find_ascii_exact(&self, bytes: &[u8], needle: &[u8], range: Range<usize>) -> Option<usize> {
        let len = needle.len();
        let window = bytes.get(range.start..range.end + len - 1)?;
        let whole = self.words == WordMatch::WholeWords;
        if self.direction.is_backward() {
            memchr::memmem::rfind_iter(window, needle)
                .map(|rel| range.start + rel)
                .find(|&col| !whole || ascii_word_ok(bytes, col, len))
        } else {
            memchr::memmem::find_iter(window, needle)
                .map(|rel| range.start + rel)
                .find(|&col| !whole || ascii_word_ok(bytes, col, len))
        }
    }

    fn find_ascii_ignore_case(
        &self,
        bytes: &[u8],
        needle_lower: &[u8],
        mut range: Range<usize>,
    ) -> Option<usize> {
        let len = needle_lower.len();
        let first = *needle_lower.first()?;
        let last = *needle_lower.last()?;
        let whole = self.words == WordMatch::WholeWords;
        let matches = |col: usize| {
            ascii_ignore_case_at(bytes, needle_lower, first, last, col)
                && (!whole || ascii_word_ok(bytes, col, len))
        };
        if self.direction.is_backward() {
            range.rev().find(|&col| matches(col))
        } else {
            range.find(|&col| matches(col))
        }
    }
}
