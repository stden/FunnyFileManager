//! Purpose: Safe SIMD ASCII anchor prefiltering for chunked Unicode content search.
//!
//! Across all 1,112,064 Unicode scalar values, the only non-ASCII code points
//! whose `char::to_lowercase()` expansion contains an ASCII byte are U+0130 (`İ` → `i\u{307}`)
//! and U+212A (`K` → `k`), and `char::REPLACEMENT_CHARACTER` encodes as `[0xEF, 0xBF, 0xBD]`.
//! Any other folded ASCII byte `< 0x80` can only originate from itself or its ASCII
//! uppercase counterpart in the raw byte stream.

use crate::casefold::{ascii_anchor_rank, has_ascii_anchor};

pub(super) fn safe_anchor(needle: &str) -> Option<u8> {
    needle
        .bytes()
        .filter(|&b| b < 0x80 && b != b'i' && b != b'k')
        .min_by_key(|&b| ascii_anchor_rank(b))
}

pub(super) fn ruled_out(anchor: Option<u8>, text: &str, bytes: &[u8]) -> bool {
    anchor.is_some_and(|a| !has_ascii_anchor(text.as_bytes(), a) && !has_ascii_anchor(bytes, a))
}

/// Byte offset where the preserved cross-chunk tail begins when the rest of
/// `bytes` has already been ruled out by [`has_ascii_anchor`].
pub(super) fn tail_cut(bytes: &[u8], needle_len: usize) -> Option<usize> {
    let window = needle_len.saturating_mul(4).saturating_add(16);
    let mut cut = bytes.len().checked_sub(window)?;
    if cut == 0 {
        return None;
    }
    let min_cut = cut.saturating_sub(3);
    while cut > min_cut && bytes.get(cut).is_some_and(|&b| (b & 0xC0) == 0x80) {
        cut -= 1;
    }
    (cut > 0).then_some(cut)
}

fn is_incomplete_utf8(invalid: &[u8]) -> bool {
    std::str::from_utf8(invalid)
        .err()
        .is_some_and(|error| error.error_len().is_none())
}

/// Keep only what a match that starts before the next chunk could use: the
/// last `needle_len - 1` bytes, widened to a character boundary.
pub(super) fn keep_tail(text: &mut String, needle_len: usize) {
    let cut = text.floor_char_boundary(text.len().saturating_sub(needle_len.saturating_sub(1)));
    text.drain(..cut);
}

pub(super) fn decode_slice(slice: &[u8], eof: bool, undecoded: &mut Vec<u8>, out: &mut String) {
    let mut rest = slice;
    for part in slice.utf8_chunks() {
        let valid = part.valid();
        if !valid.is_empty() {
            super::fold_into(valid, out);
        }
        let invalid = part.invalid();
        rest = rest.get(valid.len() + invalid.len()..).unwrap_or_default();
        if invalid.is_empty() {
            continue;
        }
        if !eof && rest.is_empty() && is_incomplete_utf8(invalid) {
            undecoded.extend_from_slice(invalid);
        } else {
            out.push(char::REPLACEMENT_CHARACTER);
        }
    }
}
