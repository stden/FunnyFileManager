//! Purpose: The scroll offsets in this program, which end each anchors to,
//! and the window of a list that a body of some height shows.
//!
//! Pure: no `App`, no I/O. The two vertical offsets are counts of lines and
//! both were bare `usize`s, but they move in **opposite directions** — the
//! file viewer counts down from the top of its content, the console transcript
//! counts up from the newest line. Naming them side by side is the point: a
//! value from one is a valid-looking, silently wrong value for the other. The
//! third, [`ScrollLeft`], is the viewer's sideways pan, counted in cells.
//!
//! [`visible_window`] lives here rather than in `ui` because more than one
//! list scrolls by it — the file panel, the folder tree and the task list —
//! and the task list's paging, in `models`, must agree with what `ui` draws.

use std::ops::Range;

/// Lines scrolled **down from the top**, as the file viewer scrolls.
///
/// A viewer opens at the beginning of a file and the reader works forwards, so
/// `TOP` is where it starts and the offset is the first visible line.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct ScrollTop(usize);

impl ScrollTop {
    /// The first line of the content.
    pub const TOP: Self = Self(0);

    /// The index of the first visible line.
    #[must_use]
    pub const fn lines(self) -> usize {
        self.0
    }

    /// Move `lines` further down, never past `last` — the clamp lives here so
    /// no caller has to remember it, which is what the viewer's key handler
    /// used to do by hand.
    #[must_use]
    pub const fn down(self, lines: usize, last: usize) -> Self {
        let moved = self.0 + lines;
        Self(if moved > last { last } else { moved })
    }

    /// Move `lines` back towards the top, stopping there.
    #[must_use]
    pub const fn up(self, lines: usize) -> Self {
        Self(self.0.saturating_sub(lines))
    }
}

/// Lines scrolled **up from the bottom**, as the console transcript scrolls.
///
/// A console shows the newest output and the reader looks backwards, so
/// `BOTTOM` is where it sits and new output always returns it there.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct ScrollBack(usize);

impl ScrollBack {
    /// The newest line — where new output always brings the view back to.
    pub const BOTTOM: Self = Self(0);

    #[must_use]
    pub const fn lines(self) -> usize {
        self.0
    }

    /// Move `lines` further back in time, never past `oldest`.
    #[must_use]
    pub const fn back(self, lines: usize, oldest: usize) -> Self {
        let moved = self.0 + lines;
        Self(if moved > oldest { oldest } else { moved })
    }

    /// Move `lines` towards the present, stopping at the newest.
    #[must_use]
    pub const fn forward(self, lines: usize) -> Self {
        Self(self.0.saturating_sub(lines))
    }

    /// The offset of the oldest kept line, for a jump to the top.
    #[must_use]
    pub const fn oldest(oldest: usize) -> Self {
        Self(oldest)
    }
}

/// Cells Far's `Ctrl+Left` and `Ctrl+Right` pan by: in the F3 viewer, in a
/// quick-view panel and in the task list's Command column.
pub const WIDE_STEP: usize = 20;

/// Cells scrolled **right from the left edge**, as the file viewer pans.
///
/// Counted in screen cells — the unit `ui::Cells` budgets a row in — because
/// Far's Left and Right move the window by columns, not by characters, and a
/// CJK glyph is one of those and two of these. There is no far end to clamp
/// to: Far lets the window run past the longest line, and stopping it would
/// need every line measured on every keypress.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct ScrollLeft(usize);

impl ScrollLeft {
    /// The left edge of the content.
    pub const LEFT: Self = Self(0);

    /// Cells hidden off the left edge.
    #[must_use]
    pub const fn cells(self) -> usize {
        self.0
    }

    /// Pan `cells` further right.
    #[must_use]
    pub const fn right(self, cells: usize) -> Self {
        Self(self.0.saturating_add(cells))
    }

    /// Pan `cells` back towards the left edge, stopping there — which is the
    /// whole of "Left at column 0 stays at 0".
    #[must_use]
    pub const fn left(self, cells: usize) -> Self {
        Self(self.0.saturating_sub(cells))
    }
}

/// A body of lines that scrolls up and down and pans left and right, the way
/// the F3 viewer and the quick view both do: one set of moves, so the two
/// cannot drift apart.
pub trait Scrolled {
    /// Where the view is: the first line shown, and the cells panned right.
    fn position(&mut self) -> (&mut ScrollTop, &mut ScrollLeft);

    /// How many lines there are to scroll through.
    fn line_count(&self) -> usize;

    /// Furthest the view may scroll on a `page`-row body: the last line stops
    /// on the bottom row, as Far's does, instead of climbing to the top and
    /// leaving the screen empty below it.
    fn max_scroll(&self, page: usize) -> usize {
        self.line_count().saturating_sub(page)
    }

    fn scroll_down(&mut self, lines: usize, page: usize) {
        let last = self.max_scroll(page);
        let (top, _) = self.position();
        *top = top.down(lines, last);
    }

    fn scroll_up(&mut self, lines: usize) {
        let (top, _) = self.position();
        *top = top.up(lines);
    }

    /// Far's Home: the first screen, back at the left edge.
    fn scroll_to_top(&mut self) {
        let (top, left) = self.position();
        *top = ScrollTop::TOP;
        *left = ScrollLeft::LEFT;
    }

    /// Far's End: the last screen, back at the left edge.
    fn scroll_to_bottom(&mut self, page: usize) {
        let last = self.max_scroll(page);
        let (top, left) = self.position();
        *top = ScrollTop::TOP.down(last, last);
        *left = ScrollLeft::LEFT;
    }

    fn scroll_left(&mut self, cells: usize) {
        let (_, left) = self.position();
        *left = left.left(cells);
    }

    fn scroll_right(&mut self, cells: usize) {
        let (_, left) = self.position();
        *left = left.right(cells);
    }
}

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

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_window;
