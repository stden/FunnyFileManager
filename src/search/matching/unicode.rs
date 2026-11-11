//! Purpose: Safe SIMD ASCII/UTF-8 anchor prefiltering and reusable buffer folding for Unicode lines.

use std::sync::OnceLock;

pub(super) use crate::casefold::has_ascii_anchor;
use crate::casefold::safe_unicode_ascii_anchor;

#[derive(Clone, Copy)]
struct Utf8Seq {
    bytes: [u8; 4],
    len: usize,
    last: u8,
}

impl Utf8Seq {
    fn of(ch: char) -> Self {
        let mut bytes = [0_u8; 4];
        let len = ch.encode_utf8(&mut bytes).len();
        let last = bytes.get(len.saturating_sub(1)).copied().unwrap_or(0);
        Self { bytes, len, last }
    }

    fn as_slice(&self) -> &[u8] {
        self.bytes.get(..self.len).unwrap_or_default()
    }
}

#[derive(Clone, Copy)]
pub(super) struct Utf8Anchor {
    primary: Utf8Seq,
    alt: Utf8Seq,
}

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
/// so column indices stay 1:1 with `text`.
pub(super) fn fold_into(text: &str, out: &mut Vec<char>) {
    out.clear();
    out.reserve(text.len().min(4096));
    out.extend(text.chars().map(fold_char_preserving_columns));
}

pub(super) fn utf8_anchor(folded_needle: &[char]) -> Option<Utf8Anchor> {
    let extras = fold_extras();
    folded_needle
        .iter()
        .copied()
        .filter(|ch| !ch.is_ascii())
        .filter_map(|target| preimage_anchor(target, extras))
        .min_by_key(|&(count, _)| count)
        .map(|(_, anchor)| anchor)
}

pub(super) fn has_utf8_anchor(bytes: &[u8], anchor: &Utf8Anchor) -> bool {
    let p_slice = anchor.primary.as_slice();
    let a_slice = anchor.alt.as_slice();
    let mut offset = 0;
    while let Some(rel) = memchr::memchr2(
        anchor.primary.last,
        anchor.alt.last,
        bytes.get(offset..).unwrap_or_default(),
    ) {
        let end = offset + rel + 1;
        let prefix = bytes.get(..end).unwrap_or_default();
        if prefix.ends_with(p_slice) || prefix.ends_with(a_slice) {
            return true;
        }
        offset = end;
    }
    false
}

fn fold_extras() -> &'static [(char, char)] {
    static EXTRAS: OnceLock<Vec<(char, char)>> = OnceLock::new();
    EXTRAS.get_or_init(|| {
        let mut pairs: Vec<(char, char)> = ('\u{80}'..=char::MAX)
            .filter_map(|raw| {
                let folded = fold_char_preserving_columns(raw);
                (folded != raw && !folded.is_ascii()).then_some((folded, raw))
            })
            .collect();
        pairs.sort_unstable();
        pairs
    })
}

fn preimage_anchor(target: char, extras: &[(char, char)]) -> Option<(usize, Utf8Anchor)> {
    let primary = Utf8Seq::of(target);
    let mut alt = primary;
    let mut count = 1_usize;
    let start = extras.partition_point(|&(folded, _)| folded < target);
    for &(folded, raw) in extras.get(start..).unwrap_or_default() {
        if folded != target {
            break;
        }
        if count >= 2 {
            return None;
        }
        alt = Utf8Seq::of(raw);
        count += 1;
    }
    Some((count, Utf8Anchor { primary, alt }))
}
