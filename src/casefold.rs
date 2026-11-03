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

/// Whether `haystack` contains `needle`, ignoring case.
///
/// Equivalent to `haystack.to_lowercase().contains(needle)` for a lowercase
/// `needle`, without the allocation. An empty needle matches everything, as
/// `str::contains` does.
pub fn contains_ignore_case(haystack: &str, needle: &str) -> bool {
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
    if let Some(anchor) = safe_unicode_ascii_anchor(needle.as_bytes()) {
        if !has_ascii_anchor(haystack.as_bytes(), anchor) {
            return None;
        }
    }
    haystack.to_lowercase().find(&needle.to_lowercase())
}

fn find_ascii_ignore_case(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    let max_start = haystack.len().checked_sub(needle.len())?;
    let (anchor_idx, anchor_low) = rarest_ascii_anchor(needle)?;
    let search = haystack.get(anchor_idx..=max_start + anchor_idx)?;
    if anchor_low.is_ascii_alphabetic() {
        for offset in memchr::memchr2_iter(anchor_low, anchor_low.to_ascii_uppercase(), search) {
            if haystack
                .get(offset..offset + needle.len())
                .is_some_and(|window| window.eq_ignore_ascii_case(needle))
            {
                return Some(offset);
            }
        }
    } else {
        for offset in memchr::memchr_iter(anchor_low, search) {
            if haystack
                .get(offset..offset + needle.len())
                .is_some_and(|window| window.eq_ignore_ascii_case(needle))
            {
                return Some(offset);
            }
        }
    }
    None
}

fn rarest_ascii_anchor(needle: &[u8]) -> Option<(usize, u8)> {
    let mut anchor_idx = 0usize;
    let mut anchor_low = needle.first().copied()?.to_ascii_lowercase();
    let mut best_rank = ascii_anchor_rank(anchor_low);
    for (idx, &byte) in needle.iter().enumerate().skip(1) {
        let low = byte.to_ascii_lowercase();
        let rank = ascii_anchor_rank(low);
        if rank < best_rank {
            anchor_idx = idx;
            anchor_low = low;
            best_rank = rank;
        }
    }
    Some((anchor_idx, anchor_low))
}

/// Rarest ASCII byte in `needle` that no non-ASCII Unicode scalar folds into
/// (`İ` -> `i\u{307}` and `K` -> `k` are excluded).
fn safe_unicode_ascii_anchor(needle: &[u8]) -> Option<u8> {
    let mut best: Option<(u8, u8)> = None;
    for &byte in needle {
        if !byte.is_ascii() {
            continue;
        }
        let low = byte.to_ascii_lowercase();
        if matches!(low, b'i' | b'k') {
            continue;
        }
        let rank = ascii_anchor_rank(low);
        if best.is_none_or(|(prev_rank, _)| rank < prev_rank) {
            best = Some((rank, low));
        }
    }
    best.map(|(_, low)| low)
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
