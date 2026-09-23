//! Purpose: The scroll offsets in this program, and which end each anchors to.
//!
//! Pure: no `App`, no I/O. The two vertical offsets are counts of lines and
//! both were bare `usize`s, but they move in **opposite directions** — the
//! file viewer counts down from the top of its content, the console transcript
//! counts up from the newest line. Naming them side by side is the point: a
//! value from one is a valid-looking, silently wrong value for the other. The
//! third, [`ScrollLeft`], is the viewer's sideways pan, counted in cells.

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

#[cfg(test)]
mod tests;
