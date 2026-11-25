//! Purpose: the Copy dialog's controls: their tab order, which are checkboxes, which are wired, and which body row draws each.

/// Which control in the dialog has the cursor.
///
/// An enum rather than an index so the tab order is stated once, here, instead
/// of being implied by arithmetic at three call sites — and so a control added
/// in the middle cannot silently renumber the rest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum CopyField {
    #[default]
    Destination,
    Rights,
    Existing,
    Timestamps,
    SymlinkContents,
    MultipleDestinations,
    Filter,
    Copy,
    Cancel,
}

impl CopyField {
    /// The tab order, top to bottom and then along the buttons.
    const ORDER: [Self; 9] = [
        Self::Destination,
        Self::Rights,
        Self::Existing,
        Self::Timestamps,
        Self::SymlinkContents,
        Self::MultipleDestinations,
        Self::Filter,
        Self::Copy,
        Self::Cancel,
    ];

    /// Tab: the next control, wrapping. A dialog is a ring — unlike a file
    /// listing, where the ends mean something.
    #[must_use]
    pub fn next(self) -> Self {
        crate::cycle::step(&Self::ORDER, self, true)
    }

    #[must_use]
    pub fn previous(self) -> Self {
        crate::cycle::step(&Self::ORDER, self, false)
    }

    /// Whether this control is a checkbox, and so answers to the space bar.
    #[must_use]
    pub const fn is_checkbox(self) -> bool {
        matches!(
            self,
            Self::Timestamps | Self::SymlinkContents | Self::MultipleDestinations | Self::Filter
        )
    }
}

impl CopyField {
    /// Whether the transfer underneath can actually carry this control out.
    ///
    /// `ops::file` reads the destination, the collision policy, the access
    /// rights, the timestamps and — since links became a choice — this
    /// dialog's link policy. "Process multiple destinations" would need a
    /// parser over the destination line and "Use filter" needs Far's filters
    /// menu; neither exists yet. Far draws a control that does not apply
    /// rather than hiding it, so both stay on the dialog, greyed, skipped by
    /// Tab and deaf to the space bar, until something behind them does. A box
    /// that ticks and quietly does nothing is worse than one that visibly
    /// cannot.
    #[must_use]
    pub const fn is_wired(self) -> bool {
        !matches!(self, Self::MultipleDestinations | Self::Filter)
    }
}

/// Which control the Copy dialog draws on body row `row`.
///
/// The dialog's row order lives here rather than only in the renderer, because
/// the mouse needs it too: a click has to reach the same control the user is
/// pointing at. Kept next to [`CopyField`] so adding a control means changing
/// one file, and covered by a test that walks the whole list.
#[must_use]
pub const fn field_at_row(row: usize) -> Option<CopyField> {
    match row {
        // 0 is the caption, 2 and 4 are rules, 9 is a rule.
        1 => Some(CopyField::Destination),
        3 => Some(CopyField::Rights),
        5 => Some(CopyField::Existing),
        6 => Some(CopyField::Timestamps),
        7 => Some(CopyField::SymlinkContents),
        8 => Some(CopyField::MultipleDestinations),
        10 => Some(CopyField::Filter),
        _ => None,
    }
}
