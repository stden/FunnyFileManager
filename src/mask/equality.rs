//! Original spelling for editing, without changing mask comparison semantics.

use super::Mask;

impl Mask {
    /// The exact expression as typed, including case and quoted separators.
    pub(crate) fn source(&self) -> &str {
        &self.source
    }

    /// Whether every possible filename satisfies this mask (e.g. `*`).
    #[must_use]
    pub fn matches_all(&self) -> bool {
        self.exclude.is_empty()
            && self
                .include
                .iter()
                .any(|p| p.raw.bytes().all(|b| b == b'*'))
    }
}

impl PartialEq for Mask {
    fn eq(&self, other: &Self) -> bool {
        self.include == other.include && self.exclude == other.exclude
    }
}

impl Eq for Mask {}
