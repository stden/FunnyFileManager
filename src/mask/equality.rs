//! Original spelling for editing, without changing mask comparison semantics.

use super::Mask;

impl Mask {
    /// The exact expression as typed, including case and quoted separators.
    pub(crate) fn source(&self) -> &str {
        &self.source
    }
}

impl PartialEq for Mask {
    fn eq(&self, other: &Self) -> bool {
        self.include == other.include && self.exclude == other.exclude
    }
}

impl Eq for Mask {}
