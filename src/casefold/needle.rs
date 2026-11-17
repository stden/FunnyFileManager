//! Purpose: Precomputed case-insensitive substring needle for repeated matching
//! across many haystacks (`KeePass` vault filter, process table filter/search).

use super::{
    find_ascii_with_anchor, has_ascii_anchor, is_ascii_safe_needle, rarest_ascii_anchor,
    safe_unicode_ascii_anchor,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Plan {
    Empty,
    AsciiSafe {
        anchor_idx: usize,
        anchor_low: u8,
    },
    /// ASCII needle containing `'i'` or `'k'`, which Unicode `'İ'` and `'K'` fold into.
    AsciiWithFoldHazards {
        anchor_idx: usize,
        anchor_low: u8,
        safe_anchor: Option<u8>,
    },
    Unicode {
        safe_anchor: Option<u8>,
    },
}

impl Plan {
    fn from_folded(bytes: &[u8]) -> Self {
        if !bytes.is_ascii() {
            return Self::Unicode {
                safe_anchor: safe_unicode_ascii_anchor(bytes),
            };
        }
        let Some((anchor_idx, anchor_low)) = rarest_ascii_anchor(bytes) else {
            return Self::Empty;
        };
        if is_ascii_safe_needle(bytes) {
            Self::AsciiSafe {
                anchor_idx,
                anchor_low,
            }
        } else {
            Self::AsciiWithFoldHazards {
                anchor_idx,
                anchor_low,
                safe_anchor: safe_unicode_ascii_anchor(bytes),
            }
        }
    }
}

/// Lowercased needle paired with its precomputed anchor strategy so scanning
/// thousands of fields pays neither per-call ASCII classification nor anchor rank.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseNeedle {
    folded: String,
    plan: Plan,
}

impl CaseNeedle {
    #[must_use]
    pub fn new(needle: &str) -> Self {
        let folded = super::folded_name(needle);
        let plan = Plan::from_folded(folded.as_bytes());
        Self { folded, plan }
    }

    #[inline]
    #[must_use]
    pub fn contains_in(&self, haystack: &str) -> bool {
        let haystack_bytes = haystack.as_bytes();
        let needle_bytes = self.folded.as_bytes();
        match self.plan {
            Plan::Empty => true,
            Plan::AsciiSafe {
                anchor_idx,
                anchor_low,
            } => find_ascii_with_anchor(haystack_bytes, needle_bytes, anchor_idx, anchor_low)
                .is_some(),
            Plan::AsciiWithFoldHazards {
                anchor_idx,
                anchor_low,
                safe_anchor,
            } => {
                find_ascii_with_anchor(haystack_bytes, needle_bytes, anchor_idx, anchor_low)
                    .is_some()
                    || (!haystack.is_ascii()
                        && self.contains_unicode_fallback(haystack, safe_anchor))
            }
            Plan::Unicode { safe_anchor } => {
                !haystack.is_ascii() && self.contains_unicode_fallback(haystack, safe_anchor)
            }
        }
    }

    fn contains_unicode_fallback(&self, haystack: &str, safe_anchor: Option<u8>) -> bool {
        safe_anchor.is_none_or(|anchor| has_ascii_anchor(haystack.as_bytes(), anchor))
            && haystack.to_lowercase().contains(&self.folded)
    }
}
