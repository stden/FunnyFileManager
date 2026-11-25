//! Purpose: Zero-allocation UTF-8 byte search for exact-case (`Case::Match`) queries.

use memchr::memmem::{Finder, FinderRev};

use super::super::{Search, WordMatch};
use super::is_word_char;

pub(super) struct ExactFinders<'a> {
    forward: Finder<'a>,
    backward: FinderRev<'a>,
    needle_chars: usize,
}

impl<'a> ExactFinders<'a> {
    pub(super) fn new(raw: &'a str) -> Self {
        Self {
            forward: Finder::new(raw.as_bytes()),
            backward: FinderRev::new(raw.as_bytes()),
            needle_chars: raw.chars().count(),
        }
    }

    pub(super) fn contains_in(&self, bytes: &[u8]) -> bool {
        self.forward.find(bytes).is_some()
    }
}

impl Search {
    pub(super) fn find_exact_utf8(
        &self,
        text: &str,
        finders: &ExactFinders<'_>,
        step: usize,
        count: usize,
        caret: usize,
    ) -> Option<usize> {
        let (min_byte, max_end_byte) =
            self.exact_byte_window(text, finders.needle_chars, step, count, caret)?;
        let window = text.as_bytes().get(min_byte..max_end_byte)?;
        let needle_len = self.needle.as_str().len();
        if self.direction.is_backward() {
            find_backward_in_window(
                self.words,
                text,
                window,
                min_byte,
                &finders.backward,
                needle_len,
            )
        } else {
            find_forward_in_window(
                self.words,
                text,
                window,
                min_byte,
                &finders.forward,
                needle_len,
            )
        }
    }

    fn exact_byte_window(
        &self,
        text: &str,
        needle_chars: usize,
        step: usize,
        count: usize,
        caret: usize,
    ) -> Option<(usize, usize)> {
        if step > 0 && step < count {
            return Some((0, text.len()));
        }
        let last_start = text.chars().count().checked_sub(needle_chars)?;
        let range = self.column_range(step, count, caret, last_start);
        if range.is_empty() {
            return None;
        }
        let start_byte = byte_at_char(text, range.start);
        let end_byte = byte_at_char(text, range.end - 1 + needle_chars);
        Some((start_byte, end_byte))
    }
}

fn find_forward_in_window(
    words: WordMatch,
    text: &str,
    window: &[u8],
    base: usize,
    finder: &Finder<'_>,
    needle_len: usize,
) -> Option<usize> {
    let mut cursor = 0;
    while let Some(rel) = window.get(cursor..).and_then(|tail| finder.find(tail)) {
        let win_pos = cursor + rel;
        let pos = base + win_pos;
        if exact_word_ok(words, text, pos, pos + needle_len) {
            return Some(text.get(..pos).map_or(0, |head| head.chars().count()));
        }
        let ch_len = text
            .get(pos..)
            .and_then(|tail| tail.chars().next())
            .map_or(1, char::len_utf8);
        cursor = win_pos + ch_len;
    }
    None
}

fn find_backward_in_window(
    words: WordMatch,
    text: &str,
    window: &[u8],
    base: usize,
    finder: &FinderRev<'_>,
    needle_len: usize,
) -> Option<usize> {
    let mut limit = window.len();
    while let Some(win_pos) = window.get(..limit).and_then(|head| finder.rfind(head)) {
        let pos = base + win_pos;
        let end_pos = pos + needle_len;
        if exact_word_ok(words, text, pos, end_pos) {
            return Some(text.get(..pos).map_or(0, |head| head.chars().count()));
        }
        let ch_len = text
            .get(..end_pos)
            .and_then(|head| head.chars().next_back())
            .map_or(1, char::len_utf8);
        limit = (win_pos + needle_len).saturating_sub(ch_len);
    }
    None
}

fn exact_word_ok(words: WordMatch, text: &str, byte_pos: usize, byte_end: usize) -> bool {
    if words == WordMatch::Anywhere {
        return true;
    }
    let before_ok = text
        .get(..byte_pos)
        .and_then(|head| head.chars().next_back())
        .is_none_or(|ch| !is_word_char(ch));
    let after_ok = text
        .get(byte_end..)
        .and_then(|tail| tail.chars().next())
        .is_none_or(|ch| !is_word_char(ch));
    before_ok && after_ok
}

fn byte_at_char(text: &str, char_idx: usize) -> usize {
    text.char_indices()
        .nth(char_idx)
        .map_or(text.len(), |(byte_idx, _)| byte_idx)
}
