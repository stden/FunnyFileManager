//! Purpose: Safe SIMD ASCII anchor prefiltering for chunked Unicode content search.
//!
//! Across all 1,112,064 Unicode scalar values, the only non-ASCII code points
//! whose `char::to_lowercase()` expansion contains an ASCII byte are U+0130 (`İ` → `i\u{307}`)
//! and U+212A (`K` → `k`), and `char::REPLACEMENT_CHARACTER` encodes as `[0xEF, 0xBF, 0xBD]`.
//! Any other folded ASCII byte `< 0x80` can only originate from itself or its ASCII
//! uppercase counterpart in the raw byte stream.

use super::folding::Folding;
use crate::casefold::{has_ascii_anchor, safe_unicode_ascii_anchor};

const MAX_UTF8_CHAR_BYTES: usize = 4;
const MAX_UTF8_CONTINUATION_BYTES: usize = MAX_UTF8_CHAR_BYTES - 1;
const ANCHOR_TAIL_SLACK_BYTES: usize = 16;

const fn is_utf8_continuation_byte(byte: u8) -> bool {
    (byte & 0b1100_0000) == 0b1000_0000
}

pub(super) fn safe_anchor(needle: &str) -> Option<u8> {
    safe_unicode_ascii_anchor(needle.as_bytes())
}

pub(super) fn ruled_out(
    anchor: Option<u8>,
    needle_has_non_ascii: bool,
    undecoded_empty: bool,
    text: &str,
    bytes: &[u8],
) -> bool {
    anchor.is_some_and(|a| !has_ascii_anchor(text.as_bytes(), a) && !has_ascii_anchor(bytes, a))
        || (needle_has_non_ascii && undecoded_empty && text.is_ascii() && bytes.is_ascii())
}

/// Byte offset where the preserved cross-chunk tail begins when the rest of
/// `bytes` has already been ruled out by [`has_ascii_anchor`].
pub(super) fn tail_cut(bytes: &[u8], needle_len: usize) -> Option<usize> {
    let window = needle_len
        .saturating_mul(MAX_UTF8_CHAR_BYTES)
        .saturating_add(ANCHOR_TAIL_SLACK_BYTES);
    let mut cut = bytes.len().checked_sub(window)?;
    let min_cut = cut.saturating_sub(MAX_UTF8_CONTINUATION_BYTES);
    while cut > min_cut
        && bytes
            .get(cut)
            .copied()
            .is_some_and(is_utf8_continuation_byte)
    {
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

pub(super) fn skip_without_anchor(
    fast_anchor: Option<u8>,
    needle_has_non_ascii: bool,
    needle_len: usize,
    bytes: &[u8],
    undecoded: &mut Vec<u8>,
    text: &mut String,
) -> bool {
    if !ruled_out(
        fast_anchor,
        needle_has_non_ascii,
        undecoded.is_empty(),
        text,
        bytes,
    ) {
        return false;
    }
    if bytes.is_empty() {
        return true;
    }
    let Some(cut) = tail_cut(bytes, needle_len) else {
        return false;
    };
    undecoded.clear();
    text.clear();
    decode_slice(
        bytes.get(cut..).unwrap_or_default(),
        false,
        undecoded,
        text,
        Folding::Unicode,
    );
    keep_tail(text, needle_len);
    true
}

pub(super) fn decode_slice(
    slice: &[u8],
    eof: bool,
    undecoded: &mut Vec<u8>,
    out: &mut String,
    folding: Folding,
) {
    let mut rest = slice;
    for part in slice.utf8_chunks() {
        let valid = part.valid();
        if !valid.is_empty() {
            folding.append(valid, out);
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

pub(super) fn push_chunk(
    read: usize,
    bytes: &[u8],
    undecoded: &mut Vec<u8>,
    text: &mut String,
    folding: Folding,
) {
    if read == 0 {
        if !undecoded.is_empty() {
            let pending = std::mem::take(undecoded);
            decode_slice(&pending, true, undecoded, text, folding);
        }
        return;
    }
    let skip = if undecoded.is_empty() {
        0
    } else {
        stitch_boundary(bytes, undecoded, text, folding)
    };
    let rest = bytes.get(skip..).unwrap_or_default();
    decode_slice(rest, false, undecoded, text, folding);
}

/// Complete a UTF-8 character split across the previous chunk's tail (`undecoded`)
/// and the start of `bytes`, returning how many leading bytes of `bytes` were consumed.
fn stitch_boundary(
    bytes: &[u8],
    undecoded: &mut Vec<u8>,
    text: &mut String,
    folding: Folding,
) -> usize {
    let pending_len = undecoded.len();
    let probe_len = bytes
        .len()
        .min(MAX_UTF8_CHAR_BYTES.saturating_sub(pending_len));
    undecoded.extend_from_slice(bytes.get(..probe_len).unwrap_or_default());
    decode_complete(undecoded, text, folding);
    if bytes.len() <= probe_len {
        return bytes.len();
    }
    let total_decoded = (pending_len + probe_len).saturating_sub(undecoded.len());
    undecoded.clear();
    total_decoded.saturating_sub(pending_len)
}

/// Decode `bytes` as `String::from_utf8_lossy` would, except that a sequence
/// cut short at the very end stays in `bytes`: the next chunk completes it.
pub(super) fn decode_complete(bytes: &mut Vec<u8>, out: &mut String, folding: Folding) {
    let source = std::mem::take(bytes);
    decode_slice(&source, false, bytes, out, folding);
}
