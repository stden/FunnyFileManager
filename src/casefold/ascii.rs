//! Purpose: Byte-level ASCII case-insensitive substring and anchor scanners.

/// Candidate start span where SIMD `memchr` amortizes setup over a plain byte loop.
const SIMD_SCAN_THRESHOLD: usize = 32;

const RANK_PUNCTUATION: u8 = 0;
const RANK_DIGIT: u8 = 1;
const RANK_RARE_LETTER: u8 = 2;
const RANK_COMMON_LETTER: u8 = 3;
const RANK_WHITESPACE: u8 = 4;

#[inline]
const fn is_unicode_fold_hazard(lower_ascii: u8) -> bool {
    matches!(lower_ascii, b'i' | b'k')
}

#[inline]
fn matches_ascii_at(haystack: &[u8], needle: &[u8], start: usize) -> bool {
    haystack
        .get(start..start + needle.len())
        .is_some_and(|window| window.eq_ignore_ascii_case(needle))
}

/// Whether `needle` is ASCII and free of `'i'`/`'I'` and `'k'`/`'K'`, the only
/// ASCII bytes produced when lowercasing non-ASCII Unicode scalars (`İ`, `K`).
#[inline]
#[must_use]
pub fn is_ascii_safe_needle(needle: &[u8]) -> bool {
    needle
        .iter()
        .all(|&b| b.is_ascii() && !is_unicode_fold_hazard(b.to_ascii_lowercase()))
}

#[inline]
#[must_use]
pub fn find_ascii_ignore_case(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if haystack.len() < needle.len().saturating_add(SIMD_SCAN_THRESHOLD) {
        let first_low = needle.first().copied()?.to_ascii_lowercase();
        return find_ascii_with_anchor(haystack, needle, 0, first_low);
    }
    let (anchor_idx, anchor_low) = rarest_ascii_anchor(needle)?;
    find_ascii_with_anchor(haystack, needle, anchor_idx, anchor_low)
}

#[inline]
#[must_use]
pub fn find_ascii_with_anchor(
    haystack: &[u8],
    needle: &[u8],
    anchor_idx: usize,
    anchor_low: u8,
) -> Option<usize> {
    let max_start = haystack.len().checked_sub(needle.len())?;
    // `search` starts at `anchor_idx`, so offset `start` inside `search` sits at
    // `haystack[start + anchor_idx]` and aligns with `haystack[start..start + needle.len()]`.
    let search = haystack.get(anchor_idx..=max_start + anchor_idx)?;
    if max_start < SIMD_SCAN_THRESHOLD {
        let anchor_up = anchor_low.to_ascii_uppercase();
        for (start, &byte) in search.iter().enumerate() {
            if (byte == anchor_low || byte == anchor_up)
                && matches_ascii_at(haystack, needle, start)
            {
                return Some(start);
            }
        }
        return None;
    }
    if anchor_low.is_ascii_alphabetic() {
        memchr::memchr2_iter(anchor_low, anchor_low.to_ascii_uppercase(), search)
            .find(|&start| matches_ascii_at(haystack, needle, start))
    } else {
        memchr::memchr_iter(anchor_low, search)
            .find(|&start| matches_ascii_at(haystack, needle, start))
    }
}

#[must_use]
pub fn rarest_ascii_anchor(needle: &[u8]) -> Option<(usize, u8)> {
    needle
        .iter()
        .enumerate()
        .map(|(idx, &byte)| (idx, byte.to_ascii_lowercase()))
        .min_by_key(|&(_, low)| ascii_anchor_rank(low))
}

/// Rarest ASCII byte in `needle` that no non-ASCII Unicode scalar folds into
/// (`İ` -> `i\u{307}` and `K` -> `k` are excluded).
#[must_use]
pub fn safe_unicode_ascii_anchor(needle: &[u8]) -> Option<u8> {
    needle
        .iter()
        .filter(|&&byte| byte.is_ascii())
        .map(|&byte| byte.to_ascii_lowercase())
        .filter(|&low| !is_unicode_fold_hazard(low))
        .min_by_key(|&low| ascii_anchor_rank(low))
}

/// Selectivity rank for a lowercase ASCII anchor byte (smaller is rarer and faster to scan).
#[must_use]
pub const fn ascii_anchor_rank(byte: u8) -> u8 {
    if byte.is_ascii_whitespace() {
        RANK_WHITESPACE
    } else if byte.is_ascii_digit() {
        RANK_DIGIT
    } else if byte.is_ascii_alphabetic() {
        match byte {
            b'q' | b'z' | b'x' | b'j' | b'v' | b'w' | b'y' | b'b' | b'f' | b'g' | b'p' | b'm' => {
                RANK_RARE_LETTER
            }
            _ => RANK_COMMON_LETTER,
        }
    } else {
        RANK_PUNCTUATION
    }
}

/// SIMD check whether `bytes` contains `anchor` (or its ASCII uppercase form when alphabetic).
#[must_use]
pub fn has_ascii_anchor(bytes: &[u8], anchor: u8) -> bool {
    if anchor.is_ascii_alphabetic() {
        memchr::memchr2(anchor, anchor.to_ascii_uppercase(), bytes).is_some()
    } else {
        memchr::memchr(anchor, bytes).is_some()
    }
}
