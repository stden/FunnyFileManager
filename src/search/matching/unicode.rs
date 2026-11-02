//! Purpose: Safe SIMD ASCII anchor prefiltering and reusable buffer folding for Unicode lines.

use crate::casefold::{ascii_anchor_rank, has_ascii_anchor as probe_anchor};

use super::super::Case;

pub(super) fn safe_ascii_anchor(raw: &str) -> Option<u8> {
    raw.chars()
        .map(|ch| ch.to_lowercase().next().unwrap_or(ch))
        .filter(|&ch| ch.is_ascii() && ch != 'i' && ch != 'k')
        .filter_map(|ch| u8::try_from(u32::from(ch)).ok())
        .min_by_key(|&byte| ascii_anchor_rank(byte))
}

pub(super) fn has_ascii_anchor(bytes: &[u8], anchor: u8) -> bool {
    probe_anchor(bytes, anchor)
}

/// Fill `out` with `text`'s characters, case-folded one character at a time
/// when `case == Case::Ignore` so column indices stay 1:1 with `text`.
pub(super) fn fold_into(case: Case, text: &str, out: &mut Vec<char>) {
    out.clear();
    out.reserve(text.len().min(256));
    match case {
        Case::Match => out.extend(text.chars()),
        Case::Ignore => out.extend(text.chars().map(|ch| {
            if ch.is_ascii() {
                ch.to_ascii_lowercase()
            } else {
                ch.to_lowercase().next().unwrap_or(ch)
            }
        })),
    }
}
