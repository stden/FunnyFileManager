//! Purpose: Which of a dialog's two answers has the cursor.

/// Which button a two-answer dialog has focus on.
///
/// Two named answers rather than a `bool` or an index: `Confirm::No` says what
/// it means at the call site, where `false` would need the reader to remember
/// which way round the dialog was built.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Confirm {
    /// The default, and what Enter starts on.
    #[default]
    Yes,
    No,
}

impl Confirm {
    /// The other answer — what Left, Right and Tab all move to, there being
    /// only two.
    #[must_use]
    pub const fn toggled(self) -> Self {
        match self {
            Self::Yes => Self::No,
            Self::No => Self::Yes,
        }
    }

    #[must_use]
    pub const fn is_yes(self) -> bool {
        matches!(self, Self::Yes)
    }
}
