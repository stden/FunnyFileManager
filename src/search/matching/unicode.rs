//! Purpose: Safe SIMD ASCII anchor prefiltering and reusable buffer folding for Unicode lines.

pub(super) use crate::casefold::has_ascii_anchor;
use crate::casefold::safe_unicode_ascii_anchor;

use super::super::Case;

pub(super) fn safe_ascii_anchor(raw: &str) -> Option<u8> {
    safe_unicode_ascii_anchor(raw.as_bytes())
}

/// Fold one character to lowercase without changing character count (`İ` stays 1 char).
fn fold_char_preserving_columns(ch: char) -> char {
    if ch.is_ascii() {
        ch.to_ascii_lowercase()
    } else {
        ch.to_lowercase().next().unwrap_or(ch)
    }
}

/// Fill `out` with `text`'s characters, case-folded one character at a time
/// when `case == Case::Ignore` so column indices stay 1:1 with `text`.
pub(super) fn fold_into(case: Case, text: &str, out: &mut Vec<char>) {
    out.clear();
    out.reserve(text.len().min(4096));
    match case {
        Case::Match => out.extend(text.chars()),
        Case::Ignore => out.extend(text.chars().map(fold_char_preserving_columns)),
    }
}
