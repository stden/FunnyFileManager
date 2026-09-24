//! Purpose: Measures, clips and pads text in terminal cells, and replaces the characters a terminal would not draw.

mod clip;
mod editor_line;

use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use std::borrow::Cow;

pub use clip::fit_name;
pub(super) use clip::{fit_path, fit_quoting_path, head_cells, tail_cells};
pub(super) use editor_line::{expand_tabs, EditorLine};

/// What a name shows where the terminal would draw nothing.
///
/// U+FFFD rather than `?`, so a name that really contains a question mark does
/// not read identically to one carrying a BEL.
pub const UNPRINTABLE: char = '\u{FFFD}';

/// `name` with every character the terminal will not draw replaced by
/// [`UNPRINTABLE`]; borrowed unchanged when there is nothing to replace, which
/// is every name but the pathological one.
///
/// A file name is data from the filesystem, not a terminal stream, and two
/// entries whose names differ only by an invisible character must not look
/// like the same file. Resolving it once, here, also keeps measuring, clipping
/// and drawing in agreement. [`crate::console::printable`] does the analogous
/// job for command output, which *is* a stream and so expands tabs instead.
pub fn printable_name(name: &str) -> Cow<'_, str> {
    if name.chars().any(invisible) {
        Cow::Owned(
            name.chars()
                .map(|ch| if invisible(ch) { UNPRINTABLE } else { ch })
                .collect(),
        )
    } else {
        Cow::Borrowed(name)
    }
}

/// Whether a character would leave no mark of its own on the screen.
///
/// Control characters, which ratatui drops outright, and the formatting
/// characters that draw nothing but change what is around them: a soft hyphen
/// or a zero-width space makes two different files read as one name, and the
/// bidi overrides can turn `report-fdp.exe` into `report-exe.pdf` on the row.
/// A name is data, so all of them are shown rather than obeyed.
fn invisible(ch: char) -> bool {
    ch.is_control()
        || matches!(ch, '\u{00AD}' | '\u{061C}' | '\u{200B}'..='\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2060}'..='\u{2064}' | '\u{2066}'..='\u{2069}' | '\u{FEFF}')
}

/// A width in terminal **cells** — the unit ratatui spends a row's budget in
/// (`Buffer::set_line` subtracts each grapheme's `cell_width`).
///
/// A newtype because this module measures three different things in `usize`:
/// bytes (for slicing), characters (the editor's cursor column) and screen
/// columns. They coincide only for ASCII, and the comments scattered through
/// this file record what happens when they are mixed up — a CJK glyph is one
/// char and two columns, so a char-budgeted span overruns its allotment and
/// pushes whatever follows off the row. `Cells` makes that a type error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Cells(pub(super) usize);

impl Cells {
    pub(crate) const ZERO: Self = Self(0);

    /// The width `text` occupies on screen.
    ///
    /// Measured over the control-free segments, because that is exactly what
    /// ratatui draws: it filters out every grapheme holding a control
    /// character, while `UnicodeWidthStr::width` scores each one a full cell.
    /// Counting them budgets a row for columns that never appear, and
    /// everything behind them slides left by that many cells.
    pub(crate) fn of(text: &str) -> Self {
        Self(
            text.split(char::is_control)
                .map(UnicodeWidthStr::width)
                .sum(),
        )
    }

    /// The width of one character. Control characters report no width and are
    /// treated as zero-cell.
    fn of_char(ch: char) -> Self {
        Self(UnicodeWidthChar::width(ch).unwrap_or(0))
    }

    /// A terminal rectangle's width, as a budget to spend.
    pub(crate) fn columns(count: u16) -> Self {
        Self(usize::from(count))
    }

    /// A width already counted in cells — for a budget being shared out, where
    /// the ratios are plain arithmetic and only the result is a width again.
    pub(crate) const fn new(count: usize) -> Self {
        Self(count)
    }

    pub(crate) const fn get(self) -> usize {
        self.0
    }

    pub(crate) const fn is_zero(self) -> bool {
        self.0 == 0
    }

    pub(crate) const fn saturating_sub(self, other: Self) -> Self {
        Self(self.0.saturating_sub(other.0))
    }
}

impl std::fmt::Display for Cells {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::ops::Add for Cells {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self(self.0 + other.0)
    }
}

impl std::iter::Sum for Cells {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        Self(iter.map(Self::get).sum())
    }
}

/// Longest prefix of `text` that fits in `width` cells, and the cells it uses.
///
/// A double-width glyph is dropped whole rather than half-drawn, so the result
/// can come up one cell short of `width`; callers pad from the returned count,
/// never from a char count.
pub(super) fn take_cells(text: &str, width: Cells) -> (String, Cells) {
    let mut taken = String::new();
    let mut used = Cells::ZERO;
    for ch in text.chars() {
        let ch_cells = Cells::of_char(ch);
        if used + ch_cells > width {
            break;
        }
        taken.push(ch);
        used = used + ch_cells;
    }
    (taken, used)
}

/// `text` with its leading `width` cells dropped, and the cells actually
/// dropped — the horizontal-scroll twin of [`take_cells`].
///
/// A double-width glyph straddling the cut is dropped whole rather than
/// half-drawn, so the second value can exceed `width` by one; callers position
/// against it, never against `width`.
pub(super) fn skip_cells(text: &str, width: Cells) -> (&str, Cells) {
    let mut rest = text.chars();
    let mut used = Cells::ZERO;
    while used < width {
        let Some(ch) = rest.next() else {
            break;
        };
        used = used + Cells::of_char(ch);
    }
    (rest.as_str(), used)
}

/// A cell count or offset as a screen coordinate, saturating at `u16::MAX`.
///
/// Callers pass values already bounded by a `u16`-sized area. Saturating keeps
/// an unbounded one at the far edge; `as u16` would wrap it to a small, wrong
/// coordinate near the top-left corner.
pub(super) fn screen_u16(count: usize) -> u16 {
    u16::try_from(count).unwrap_or(u16::MAX)
}

#[cfg(test)]
mod tests;
