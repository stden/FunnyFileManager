//! Purpose: case-insensitive comparison without allocating a lowercase copy.
//!
//! Every incremental search in this program runs its test once per candidate
//! per keystroke: the vault filter over every entry's title, username, URL and
//! group path; `Alt+letter` over every name in the panel; the same over every
//! row of the folder tree.
//!
//! # Unicode
//!
//! The fast path is bytewise and therefore ASCII-only. Case folding is not a
//! byte operation in general — lowercasing `İ` yields two chars, and Turkish
//! `I` folds differently from English `I` — so anything non-ASCII falls back to
//! `to_lowercase()` once a safe ASCII anchor check cannot rule out a match.

mod ascii;
mod needle;

pub(crate) use ascii::{
    ascii_anchor_rank, find_ascii_ignore_case, find_ascii_with_anchor, has_ascii_anchor,
    is_ascii_safe_needle, rarest_ascii_anchor, safe_unicode_ascii_anchor,
};
pub(crate) use needle::CaseNeedle;

/// Whether `a` and `b` fold to the same lowercase text, without allocating.
#[must_use]
pub(crate) fn eq_ignore_case(a: &str, b: &str) -> bool {
    if a.is_ascii() && b.is_ascii() {
        a.eq_ignore_ascii_case(b)
    } else {
        a.chars()
            .flat_map(char::to_lowercase)
            .eq(b.chars().flat_map(char::to_lowercase))
    }
}

/// Whether `haystack` contains `needle`, ignoring case.
///
/// Equivalent to `haystack.to_lowercase().contains(needle)` for a lowercase
/// `needle`, without the allocation. An empty needle matches everything, as
/// `str::contains` does.
#[inline]
pub fn contains_ignore_case(haystack: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return true;
    }
    let nbytes = needle.as_bytes();
    if is_ascii_safe_needle(nbytes) || (nbytes.is_ascii() && haystack.is_ascii()) {
        return find_ascii_ignore_case(haystack.as_bytes(), nbytes).is_some();
    }
    find_ignore_case(haystack, needle).is_some()
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
/// SIMD anchor scanning + `eq_ignore_ascii_case` and does not build strings.
#[must_use]
pub(crate) fn find_ignore_case(haystack: &str, needle: &str) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    if haystack.is_ascii() && needle.is_ascii() {
        return find_ascii_ignore_case(haystack.as_bytes(), needle.as_bytes());
    }
    if is_ascii_safe_needle(needle.as_bytes()) {
        let raw = find_ascii_ignore_case(haystack.as_bytes(), needle.as_bytes())?;
        // Non-ASCII characters before `raw` can expand when lowercased (`İ` -> 3 bytes).
        return Some(folded_prefix_len(haystack.get(..raw).unwrap_or_default()));
    }
    if let Some(anchor) = safe_unicode_ascii_anchor(needle.as_bytes()) {
        if !has_ascii_anchor(haystack.as_bytes(), anchor) {
            return None;
        }
    }
    haystack.to_lowercase().find(&needle.to_lowercase())
}

fn folded_prefix_len(prefix: &str) -> usize {
    if prefix.is_ascii() {
        prefix.len()
    } else {
        prefix
            .chars()
            .flat_map(char::to_lowercase)
            .map(char::len_utf8)
            .sum()
    }
}

/// Whether `haystack` begins with `needle`, ignoring case.
pub fn starts_with_ignore_case(haystack: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return true;
    }
    if needle.is_ascii() && (haystack.is_ascii() || is_ascii_safe_needle(needle.as_bytes())) {
        return haystack
            .as_bytes()
            .get(..needle.len())
            .is_some_and(|head| head.eq_ignore_ascii_case(needle.as_bytes()));
    }
    let mut hay = haystack.chars().flat_map(char::to_lowercase);
    needle
        .chars()
        .flat_map(char::to_lowercase)
        .all(|wanted| hay.next() == Some(wanted))
}

#[cfg(test)]
mod tests;
