//! Purpose: The columns Far lays out behind a panel's names — Size, Date and
//! Time — with their widths and titles, which of them each table view shows,
//! and which the info row adds instead.

use super::super::Cells;
use super::fields::{right_cells, size_field};
use crate::models::{DirEntry, PanelView};

/// Right-aligned width of the Size column: "Folder", "Up", or a byte count.
///
/// Exactly six cells, because that is the width of the word `Folder` — the
/// column is sized to its widest label, so "Folder" fills it edge to edge with
/// no stray leading blank, and every number right-aligns against the same edge.
pub(super) const SIZE_COLUMN_WIDTH: Cells = Cells(6);

/// Width of the Date column, which is exactly `MM/DD/YY`.
const DATE_COLUMN_WIDTH: Cells = Cells(8);

/// Width of the Time column, which is exactly `HH:MM`.
const TIME_COLUMN_WIDTH: Cells = Cells(5);

/// The rule Far draws between panel columns, and the one cell it costs.
pub(in crate::ui) const COLUMN_SEPARATOR: &str = "\u{2502}";
pub(super) const COLUMN_SEPARATOR_WIDTH: Cells = Cells(1);

/// The columns Far lays out to the right of the name, in its own order.
///
/// A slice rather than three booleans because they are dropped as a group from
/// the right on a narrow panel (see [`panel_layout`]), and an ordered list is
/// what "drop the last one" is a statement about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::ui) enum PanelColumn {
    Size,
    Date,
    Time,
}

impl PanelColumn {
    pub(super) const fn width(self) -> Cells {
        match self {
            Self::Size => SIZE_COLUMN_WIDTH,
            Self::Date => DATE_COLUMN_WIDTH,
            Self::Time => TIME_COLUMN_WIDTH,
        }
    }

    pub(super) const fn title(self) -> &'static str {
        match self {
            Self::Size => "Size",
            Self::Date => "Date",
            Self::Time => "Time",
        }
    }

    /// This column's text for one row.
    pub(super) fn cell(self, entry: &DirEntry) -> String {
        match self {
            Self::Size => size_field(entry),
            Self::Date => entry.modified.date(),
            Self::Time => entry.modified.time(),
        }
    }

    /// Place `cell` in this column's cells.
    ///
    /// Every column right-aligns and every column is sized to its content, so
    /// nothing carries a stray blank: `Folder` and `MM/DD/YY` and `HH:MM` each
    /// fill their column exactly, and a byte count lines its digits up against
    /// the same right edge.
    pub(super) fn lay_out(self, cell: &str) -> String {
        right_cells(cell, self.width())
    }
}

/// `static`, not `const`: the layout hands out `&'static [PanelColumn]`
/// sub-slices of it, and indexing a `const` would borrow a temporary.
static PANEL_COLUMNS: [PanelColumn; 3] = [PanelColumn::Size, PanelColumn::Date, PanelColumn::Time];

/// The columns each of Far's table views puts behind every name, or `None`
/// for a view [`modes`] draws instead.
///
/// Brief and Medium have none — their names take the width — and the info
/// row shows the current entry's instead (see [`info_columns`]).
pub(super) fn table_columns(view: PanelView) -> Option<&'static [PanelColumn]> {
    match view {
        PanelView::Brief | PanelView::Medium => Some(&[]),
        PanelView::Full => Some(&PANEL_COLUMNS),
        PanelView::Wide => Some(&PANEL_COLUMNS[..1]),
        PanelView::Detailed
        | PanelView::Descriptions
        | PanelView::LongDescriptions
        | PanelView::Owners
        | PanelView::Links
        | PanelView::Alternative => None,
    }
}

/// What the info row adds after the current name: the columns a view's table
/// leaves out. All three under Brief and Medium, Date and Time under Wide,
/// nothing under Full — the rule Far's own status line follows.
pub(super) fn info_columns(shown: &[PanelColumn]) -> Vec<PanelColumn> {
    PANEL_COLUMNS
        .iter()
        .copied()
        .filter(|column| !shown.contains(column))
        .collect()
}
