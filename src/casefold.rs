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
        return haystack.to_lowercase().starts_with(&needle.to_lowercase());
    }
    let (haystack, needle) = (haystack.as_bytes(), needle.as_bytes());
    haystack
        .get(..needle.len())
        .is_some_and(|head| head.eq_ignore_ascii_case(needle))
}

#[cfg(test)]
mod tests;
