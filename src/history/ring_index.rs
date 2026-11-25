//! Purpose: A position in the history ring, counted back from the newest entry.

/// A position in the history ring, counted back from the newest entry.
///
/// A newtype because `History::recall` and `History::search_back` both take
/// "how far back", and so does `ConsoleLog`'s scroll — three `usize`s measuring
/// different rings that a plain integer lets you hand to the wrong one. The
/// step methods keep the walk in one place too, so `Up` and `Down` cannot drift
/// apart in how they move.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct RingIndex(usize);

impl RingIndex {
    /// The most recent entry.
    pub const NEWEST: Self = Self(0);

    #[must_use]
    pub const fn new(offset: usize) -> Self {
        Self(offset)
    }

    /// One step further back in time.
    #[must_use]
    pub const fn older(self) -> Self {
        Self(self.0 + 1)
    }

    /// One step towards the present, or `None` at the newest entry — which is
    /// what tells `Down` to restore the line the user was typing.
    #[must_use]
    pub const fn newer(self) -> Option<Self> {
        match self.0.checked_sub(1) {
            Some(offset) => Some(Self(offset)),
            None => None,
        }
    }

    #[must_use]
    pub const fn offset(self) -> usize {
        self.0
    }
}
