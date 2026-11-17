//! Purpose: Precomputed Boyer-Moore-Horspool ASCII skip table and rare-anchor
//! Unicode matcher for MCP content search.

pub(super) struct PreparedPattern {
    lower: String,
    safe_anchor: Option<u8>,
    shifts: [usize; 256],
}

impl PreparedPattern {
    pub(super) fn new(pattern_lower: &str) -> Self {
        let bytes = pattern_lower.as_bytes();
        Self {
            lower: pattern_lower.to_owned(),
            safe_anchor: rarest_safe_ascii_anchor(bytes),
            shifts: build_shifts(bytes),
        }
    }

    pub(super) fn matches(&self, haystack: &str, lower_buf: &mut String) -> bool {
        if self.lower.is_empty() {
            return true;
        }
        let needle_bytes = self.lower.as_bytes();
        if haystack.is_ascii() && self.lower.is_ascii() {
            return contains_ascii_horspool(haystack.as_bytes(), needle_bytes, &self.shifts);
        }
        if let Some(anchor) = self.safe_anchor {
            let up = anchor.to_ascii_uppercase();
            if !haystack.as_bytes().iter().any(|&b| b == anchor || b == up) {
                return false;
            }
        }
        lower_buf.clear();
        lower_buf.extend(haystack.chars().flat_map(char::to_lowercase));
        lower_buf.contains(&self.lower)
    }
}

fn contains_ascii_horspool(haystack: &[u8], needle_lower: &[u8], shifts: &[usize; 256]) -> bool {
    let len = needle_lower.len();
    let Some(max_start) = haystack.len().checked_sub(len) else {
        return false;
    };
    let last_wanted = needle_lower.last().copied().unwrap_or_default();
    let mut idx = 0;
    while idx <= max_start {
        let tail_byte = haystack.get(idx + len - 1).copied().unwrap_or_default();
        if tail_byte.to_ascii_lowercase() == last_wanted
            && haystack
                .get(idx..idx + len)
                .is_some_and(|win| win.eq_ignore_ascii_case(needle_lower))
        {
            return true;
        }
        idx += shifts.get(usize::from(tail_byte)).copied().unwrap_or(1);
    }
    false
}

fn build_shifts(needle_lower: &[u8]) -> [usize; 256] {
    let len = needle_lower.len();
    let mut shifts = [len.max(1); 256];
    for (idx, &byte) in needle_lower.iter().enumerate().take(len.saturating_sub(1)) {
        let step = len - 1 - idx;
        if let Some(slot) = shifts.get_mut(usize::from(byte)) {
            *slot = step;
        }
        if let Some(slot) = shifts.get_mut(usize::from(byte.to_ascii_uppercase())) {
            *slot = step;
        }
    }
    shifts
}

fn rarest_safe_ascii_anchor(needle_lower: &[u8]) -> Option<u8> {
    needle_lower
        .iter()
        .copied()
        .filter(|&b| b.is_ascii() && !matches!(b, b'i' | b'k'))
        .min_by_key(|&b| anchor_rank(b))
}

const fn anchor_rank(byte: u8) -> u8 {
    if byte.is_ascii_alphabetic() {
        2
    } else if byte == b' ' {
        3
    } else {
        1
    }
}
