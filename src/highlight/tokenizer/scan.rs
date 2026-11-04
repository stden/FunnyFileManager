//! Purpose: ASCII byte-slice fast paths for the built-in line tokenizer, falling
//! back to UTF-8 character iteration only when non-ASCII bytes are encountered.

use super::run_length;

#[inline]
pub(super) const fn is_ascii_ws(byte: u8) -> bool {
    matches!(byte, b'\t'..=b'\r' | b' ')
}

#[inline]
fn scan_ascii_then_utf8(
    rest: &str,
    keep_ascii: impl Fn(u8) -> bool,
    keep_utf8: impl Fn(char) -> bool,
) -> usize {
    let bytes = rest.as_bytes();
    let ascii_len = bytes.iter().take_while(|&&b| keep_ascii(b)).count();
    if bytes.get(ascii_len).is_some_and(|&b| !b.is_ascii()) {
        let tail = rest.get(ascii_len..).unwrap_or_default();
        return ascii_len + run_length(tail, keep_utf8);
    }
    ascii_len
}

pub(super) fn scan_whitespace(rest: &str) -> usize {
    scan_ascii_then_utf8(rest, is_ascii_ws, char::is_whitespace)
}

pub(super) fn scan_number(rest: &str) -> usize {
    scan_ascii_then_utf8(
        rest,
        |b| b.is_ascii_alphanumeric() || b == b'.' || b == b'_',
        |c| c.is_alphanumeric() || c == '.' || c == '_',
    )
}

pub(super) fn scan_word(rest: &str, word_hyphens: bool) -> usize {
    scan_ascii_then_utf8(
        rest,
        |b| b.is_ascii_alphanumeric() || b == b'_' || (word_hyphens && b == b'-'),
        |c| c.is_alphanumeric() || c == '_' || (word_hyphens && c == '-'),
    )
}

/// Byte length of the quoted string starting at `text[0]`, including both
/// quotes — or the rest of the line when the closing quote is missing, which
/// an editor sees constantly because the user is still typing it.
pub(super) fn string_length(text: &str, quote: u8) -> usize {
    let bytes = text.as_bytes();
    let mut search_from = 1;
    loop {
        let remaining = bytes.get(search_from..).unwrap_or_default();
        let Some(rel_pos) = memchr::memchr2(quote, b'\\', remaining) else {
            return text.len();
        };
        let hit = search_from + rel_pos;
        let after_hit = hit + 1;
        if bytes.get(hit) == Some(&quote) {
            return after_hit;
        }
        let escaped_len = text
            .get(after_hit..)
            .and_then(|s| s.chars().next())
            .map_or(0, char::len_utf8);
        search_from = after_hit + escaped_len;
    }
}
