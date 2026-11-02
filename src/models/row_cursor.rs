//! Purpose: The cursor of a plain list of rows — the sessions panel, the
//! remote panel and the F11 plugin commands — and how the list keys move it.
//!
//! One type rather than three copies of `next` / `previous` / `page_down`:
//! the clamp at each end is the part every hand-written copy gets wrong, and
//! a cursor that cannot leave its list is the invariant all three rely on.

/// A move a list key asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowMotion {
    Previous,
    Next,
    First,
    Last,
    /// `PgUp`: this many rows up.
    PageUp(usize),
    /// `PgDn`: this many rows down.
    PageDown(usize),
}

/// Which row of a list has the cursor. Always inside the list it moved in:
/// every move clamps to the rows it is given.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RowCursor(usize);

impl RowCursor {
    /// The first row.
    pub const TOP: Self = Self(0);

    /// The cursor on `index`, clamped to a list of `rows` rows.
    #[must_use]
    pub fn at(index: usize, rows: usize) -> Self {
        Self(clamp(index, rows))
    }

    #[must_use]
    pub const fn index(self) -> usize {
        self.0
    }

    /// Where `motion` takes the cursor in a list of `rows` rows. A page is at
    /// least one row, so a panel too short to hold a page still moves.
    #[must_use]
    pub fn moved(self, motion: RowMotion, rows: usize) -> Self {
        let at = clamp(self.0, rows);
        let index = match motion {
            RowMotion::Previous => at.saturating_sub(1),
            RowMotion::Next => at.saturating_add(1),
            RowMotion::First => 0,
            RowMotion::Last => rows,
            RowMotion::PageUp(page) => at.saturating_sub(at_least_one(page)),
            RowMotion::PageDown(page) => at.saturating_add(at_least_one(page)),
        };
        Self(clamp(index, rows))
    }
}

/// `index` within `0..rows`, or 0 for an empty list.
fn clamp(index: usize, rows: usize) -> usize {
    index.min(rows.saturating_sub(1))
}

/// A page of at least one row, so a panel too short for a page still moves.
fn at_least_one(page: usize) -> usize {
    page.max(1)
}

#[cfg(test)]
mod tests;
