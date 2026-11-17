//! Purpose: Case-folded sort keys and allocation-free tie-break comparison.

use std::cmp::Ordering;

/// Pre-folded sort key: ASCII borrows the original slice and folds bytes on the fly,
/// while non-ASCII stores its full Unicode lowercase expansion once up front.
pub(super) enum Folded<'a> {
    BorrowedAscii(&'a str),
    LowercasedUnicode(String),
}

impl PartialEq for Folded<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for Folded<'_> {}

impl PartialOrd for Folded<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Folded<'_> {
    fn cmp(&self, other: &Self) -> Ordering {
        cmp_folded(self, other)
    }
}

pub(super) fn fold(name: &str) -> Folded<'_> {
    if name.is_ascii() {
        Folded::BorrowedAscii(name)
    } else {
        Folded::LowercasedUnicode(name.to_lowercase())
    }
}

pub(super) fn cmp_folded(left: &Folded<'_>, right: &Folded<'_>) -> Ordering {
    ascii_cmp(stored_bytes(left), stored_bytes(right))
}

pub(super) fn cmp_names_folded(left: &str, right: &str) -> Ordering {
    if left.is_ascii() && right.is_ascii() {
        ascii_cmp(left.as_bytes(), right.as_bytes())
    } else {
        left.chars()
            .flat_map(char::to_lowercase)
            .cmp(right.chars().flat_map(char::to_lowercase))
    }
}

fn stored_bytes<'a>(folded: &'a Folded<'_>) -> &'a [u8] {
    match folded {
        Folded::BorrowedAscii(text) => text.as_bytes(),
        Folded::LowercasedUnicode(text) => text.as_bytes(),
    }
}

/// Compare two UTF-8 byte slices while lowercasing ASCII bytes (`0x41..=0x5A`).
///
/// Multi-byte UTF-8 sequences use only bytes `>= 0x80` (untouched by `to_ascii_lowercase`),
/// and UTF-8 preserves code-point order byte-for-byte, so byte order matches `char` order.
fn ascii_cmp(left: &[u8], right: &[u8]) -> Ordering {
    left.iter()
        .copied()
        .map(|byte| byte.to_ascii_lowercase())
        .cmp(right.iter().copied().map(|byte| byte.to_ascii_lowercase()))
}
