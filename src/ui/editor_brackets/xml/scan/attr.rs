//! Purpose: SIMD `memchr` attribute and quote scanning inside open XML/HTML tags.

use super::super::TagSpan;
use super::{Mode, OpenTag, ScanStep, XmlScanner};

impl ScanStep {
    pub(super) const fn ascii(len: usize) -> Self {
        Self {
            bytes: len,
            chars: len,
            pair: None,
        }
    }

    pub(super) fn utf8(rest: &str, bytes: usize) -> Self {
        Self {
            bytes: bytes.max(1),
            chars: chars_in(rest, bytes).max(1),
            pair: None,
        }
    }

    pub(super) const fn matched(bytes: usize, chars: usize, pair: [TagSpan; 2]) -> Self {
        Self {
            bytes,
            chars,
            pair: Some(pair),
        }
    }
}

impl<'a> XmlScanner<'a> {
    pub(super) fn step_open_tag(
        &mut self,
        line_idx: usize,
        char_pos: usize,
        rest: &'a str,
    ) -> ScanStep {
        let Mode::OpenTag {
            pending,
            ref mut quote,
        } = self.mode
        else {
            return ScanStep::ascii(1);
        };
        let bytes = rest.as_bytes();
        if let Some(q) = *quote {
            if let Some(end) = memchr::memchr(q, bytes) {
                *quote = None;
                return ScanStep::utf8(rest, end + 1);
            }
            return ScanStep::utf8(rest, rest.len());
        }
        let Some(hit) = memchr::memchr3(b'"', b'\'', b'>', bytes) else {
            return ScanStep::utf8(rest, rest.len());
        };
        if bytes.get(hit) == Some(&b'>') {
            return self.finish_open_tag(line_idx, char_pos, rest, hit, pending);
        }
        *quote = bytes.get(hit).copied();
        ScanStep::utf8(rest, hit + 1)
    }

    fn finish_open_tag(
        &mut self,
        line_idx: usize,
        char_pos: usize,
        rest: &'a str,
        hit: usize,
        pending: OpenTag<'a>,
    ) -> ScanStep {
        if hit > 0 && rest.as_bytes().get(hit - 1) == Some(&b'/') {
            let before_slash = chars_in(rest, hit - 1);
            let slash_col = char_pos + before_slash;
            let close = TagSpan::new(line_idx, slash_col, slash_col + 2);
            self.mode = Mode::Text;
            return ScanStep::matched(hit + 1, before_slash + 2, [pending.span, close]);
        }
        self.stack.push(pending);
        self.mode = Mode::Text;
        ScanStep::utf8(rest, hit + 1)
    }
}

fn chars_in(text: &str, bytes: usize) -> usize {
    let s = text.get(..bytes).unwrap_or(text);
    if s.is_ascii() {
        s.len()
    } else {
        s.chars().count()
    }
}

pub(super) fn ascii_run(text: &str, keep: fn(char) -> bool) -> usize {
    text.bytes()
        .take_while(|&b| b.is_ascii() && keep(char::from(b)))
        .count()
}

fn is_tag_name_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | ':')
}

pub(in crate::ui::editor_brackets::xml) fn tag_name_len(text: &str) -> usize {
    ascii_run(text, is_tag_name_char)
}
