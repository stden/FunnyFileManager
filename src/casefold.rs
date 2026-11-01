//! Purpose: case-insensitive comparison without allocating a lowercase copy.
//!
//! Every incremental search in this program runs its test once per candidate
//! per keystroke: the vault filter over every entry's title, username, URL and
//! group path; `Alt+letter` over every name in the panel; the same over every
//! row of the folder tree. The obvious spelling,
//! `candidate.to_lowercase().starts_with(&needle)`, allocates a `String` for
//! every candidate on every key — work proportional to the listing, thrown
//! away before the frame is drawn.
//!
//! Benchmarks on a 13th-gen desktop, before this module existed
//! (`benches/kdbx_bench.rs`, `benches/panel_bench.rs`):
//!
//! | path | size | before |
//! |---|---|---|
//! | `vault/filter/none` | 5 000 entries | 360 µs |
//! | `panel/quick_search/miss` | 10 000 entries | 117 µs |
//!
//! Both are per keystroke, both on the event loop, and neither draws anything
//! until it returns.
//!
//! # Unicode
//!
//! The fast path is bytewise and therefore ASCII-only. Case folding is not a
//! byte operation in general — lowercasing `İ` yields two chars, and Turkish
//! `I` folds differently from English `I` — so anything non-ASCII falls back to
//! the allocating expression these functions replace. The answer is then
//! exactly what it was before, at exactly the old cost; only the common case
//! gets faster.
//!
//! The needle is expected to be lowercased already where the caller can hoist
//! that out of its loop (the vault does); these functions do not require it,
//! because they compare case-insensitively in both directions.

/// Whether `haystack` contains `needle`, ignoring case.
///
/// Equivalent to `haystack.to_lowercase().contains(needle)` for a lowercase
/// `needle`, without the allocation. An empty needle matches everything, as
/// `str::contains` does.
pub fn contains_ignore_case(haystack: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return true;
    }
    if !haystack.is_ascii() || !needle.is_ascii() {
        return haystack.to_lowercase().contains(&needle.to_lowercase());
    }
    let (haystack, needle) = (haystack.as_bytes(), needle.as_bytes());
    haystack.len() >= needle.len()
        && haystack
            .windows(needle.len())
            .any(|window| window.eq_ignore_ascii_case(needle))
}

/// Lowercase form shared by ordering and prefix search.
///
/// ASCII takes `to_ascii_lowercase`, which matches `to_lowercase` on ASCII.
/// Any other scalar uses `to_lowercase`, so expansions such as `İ` stay intact.
#[must_use]
pub(crate) fn folded_name(name: &str) -> String {
    if name.is_ascii() {
        name.to_ascii_lowercase()
    } else {
        name.to_lowercase()
    }
}

/// Byte index of `needle` in `haystack`, ignoring case.
///
/// The index is the one `haystack.to_lowercase().find(&needle.to_lowercase())`
/// returns, including when lowercase expands a character. ASCII finds it with
/// `eq_ignore_ascii_case` and does not build those strings.
#[must_use]
pub(crate) fn find_ignore_case(haystack: &str, needle: &str) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    if !haystack.is_ascii() || !needle.is_ascii() {
        return haystack.to_lowercase().find(&needle.to_lowercase());
    }
    let (haystack, needle) = (haystack.as_bytes(), needle.as_bytes());
    haystack
        .windows(needle.len())
        .position(|window| window.eq_ignore_ascii_case(needle))
}

/// Whether `haystack` begins with `needle`, ignoring case.
///
/// Equivalent to `haystack.to_lowercase().starts_with(needle)` for a lowercase
/// `needle`, without the allocation. An empty needle matches everything, as
/// `str::starts_with` does.
pub fn starts_with_ignore_case(haystack: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return true;
    }
    if !haystack.is_ascii() || !needle.is_ascii() {
        let mut hay = haystack.chars().flat_map(char::to_lowercase);
        return needle
            .chars()
            .flat_map(char::to_lowercase)
            .all(|wanted| hay.next() == Some(wanted));
    }
    let (haystack, needle) = (haystack.as_bytes(), needle.as_bytes());
    haystack
        .get(..needle.len())
        .is_some_and(|head| head.eq_ignore_ascii_case(needle))
}

/// Selectivity rank for a lowercase ASCII anchor byte (smaller is rarer and faster to scan).
#[must_use]
pub(crate) const fn ascii_anchor_rank(byte: u8) -> u8 {
    if byte.is_ascii_whitespace() {
        4
    } else if byte.is_ascii_digit() {
        1
    } else if byte.is_ascii_alphabetic() {
        match byte {
            b'q' | b'z' | b'x' | b'j' | b'v' | b'w' | b'y' | b'b' | b'f' | b'g' | b'p' | b'm' => 2,
            _ => 3,
        }
    } else {
        0
    }
}

/// SIMD check whether `bytes` contains `anchor` (or its ASCII uppercase form when alphabetic).
#[must_use]
pub(crate) fn has_ascii_anchor(bytes: &[u8], anchor: u8) -> bool {
    if anchor.is_ascii_alphabetic() {
        memchr::memchr2(anchor, anchor.to_ascii_uppercase(), bytes).is_some()
    } else {
        memchr::memchr(anchor, bytes).is_some()
    }
}

#[cfg(test)]
mod tests;
