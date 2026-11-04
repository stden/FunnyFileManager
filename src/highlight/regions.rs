//! Purpose: Lays Colorer's regions over one line as token runs.

use super::{colorer, Token};

enum CharOffsets {
    Ascii(usize),
    Utf8(Vec<usize>),
}

impl CharOffsets {
    fn of(line: &str) -> Self {
        if line.is_ascii() {
            return Self::Ascii(line.len());
        }
        let offsets: Vec<usize> = line
            .char_indices()
            .map(|(at, _)| at)
            .chain(std::iter::once(line.len()))
            .collect();
        Self::Utf8(offsets)
    }

    fn char_count(&self) -> usize {
        match self {
            Self::Ascii(len) => *len,
            Self::Utf8(offsets) => offsets.len().saturating_sub(1),
        }
    }

    fn byte_range(&self, char_start: usize, char_end: usize) -> std::ops::Range<usize> {
        match self {
            Self::Ascii(_) => char_start..char_end,
            Self::Utf8(offsets) => {
                let from = offsets.get(char_start).copied().unwrap_or(0);
                let to = offsets.get(char_end).copied().unwrap_or(from);
                from..to
            }
        }
    }
}

/// Lay Colorer's regions over a line, filling the gaps with plain text.
///
/// Colorer reports overlapping regions — a bracket is both `def:Symbol` and
/// `def:PairStart` — so later regions win, which is Colorer's own rule. The
/// result covers the whole line so the caller can concatenate it back.
pub(super) fn paint<'a>(line: &'a str, spans: &[colorer::Span]) -> Vec<(&'a str, Token)> {
    let offsets = CharOffsets::of(line);
    let width = offsets.char_count();
    let mut per_char = vec![Token::Normal; width];
    for span in spans {
        for slot in per_char
            .iter_mut()
            .take(span.end.min(width))
            .skip(span.start.min(width))
        {
            *slot = span.token;
        }
    }
    let mut runs = Vec::new();
    let mut char_pos = 0;
    for group in per_char.chunk_by(|a, b| a == b) {
        let next_pos = char_pos + group.len();
        let bytes = offsets.byte_range(char_pos, next_pos);
        if let Some((&token, text)) = group.first().zip(line.get(bytes)) {
            runs.push((text, token));
        }
        char_pos = next_pos;
    }
    runs
}

/// Convert a UTF-16 code-unit offset (from Colorer's `UnicodeString`) into a
/// Rust `char` index on `line`.
pub(super) fn utf16_to_char_idx(line: &str, utf16_idx: usize) -> usize {
    if line.is_ascii() {
        return utf16_idx.min(line.len());
    }
    let mut utf16_offset = 0;
    for (char_idx, ch) in line.chars().enumerate() {
        if utf16_offset >= utf16_idx {
            return char_idx;
        }
        utf16_offset += ch.len_utf16();
    }
    line.chars().count()
}
