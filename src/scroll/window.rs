//! Purpose: Which rows of a list a viewport actually shows.

use std::ops::Range;

/// Which items of a list `len` long a body `rows` tall will actually show.
///
/// The same rule ratatui's `List` applies to its own state, worked out before
/// the items are built rather than after: the offset holds unless the cursor
/// has moved outside it, and then it moves by just enough to bring the cursor
/// back.
///
/// Returned as a range so the caller can slice with it, and pure so that the
/// scrolling behaviour can be checked without a terminal. The sums saturate:
/// a release build does not check overflow, and a wrapped end would show a
/// window that has nothing to do with the cursor.
#[must_use]
pub fn visible_window(
    len: usize,
    rows: usize,
    offset: usize,
    selected: Option<usize>,
) -> Range<usize> {
    if len == 0 || rows == 0 {
        return 0..0;
    }
    let mut start = offset.min(len.saturating_sub(1));
    if let Some(selected) = selected.filter(|at| *at < len) {
        if selected < start {
            // Scrolled up past the cursor: the cursor becomes the first row.
            start = selected;
        } else if selected >= start.saturating_add(rows) {
            // …and down past it: the cursor becomes the last.
            start = (selected + 1).saturating_sub(rows);
        }
    }
    // A short listing is shown from the top rather than with blank rows above
    // it, which is what a stale offset into a directory that just shrank would
    // otherwise leave behind.
    start = start.min(len.saturating_sub(rows));
    start..start.saturating_add(rows).min(len)
}
