//! Purpose: Measures, clips and pads text in terminal cells, and replaces the characters a terminal would not draw.

use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use std::borrow::Cow;

use crate::models::{CharColumn, TabSize};

use super::panel::pad_cells;

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
pub fn printable_name(name: &str) -> std::borrow::Cow<'_, str> {
    if name.chars().any(invisible) {
        std::borrow::Cow::Owned(
            name.chars()
                .map(|ch| if invisible(ch) { UNPRINTABLE } else { ch })
                .collect(),
        )
    } else {
        std::borrow::Cow::Borrowed(name)
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
    pub(crate) const fn columns(count: u16) -> Self {
        Self(count as usize)
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
    let mut used = Cells::ZERO;
    for (index, ch) in text.char_indices() {
        if used >= width {
            return (&text[index..], used);
        }
        used = used + Cells::of_char(ch);
    }
    ("", used)
}

/// Cells occupied by the first `column` characters of the editor line `text`,
/// each TAB reaching the next stop of `tab`.
///
/// Bridges the editor's char-indexed caret (right for editing) to the screen
/// column it actually lands on (right for drawing) — the one place the two
/// units legitimately meet, which is why both the conversion and its input
/// have names.
fn cells_before(text: &str, column: CharColumn, tab: TabSize) -> Cells {
    text.chars()
        .take(column.chars())
        .fold(Cells::ZERO, |at, ch| advance(at, ch, tab))
}

/// `text` as the editor draws it when it starts at cell `start`: every TAB
/// widened to the spaces that reach the next stop of `tab`.
///
/// Borrowed when there is no TAB, which is almost every line. A TAB used to
/// reach the screen as itself; ratatui drops control characters, so it drew
/// as nothing and everything after it slid left.
pub(super) fn expand_tabs(text: &str, start: Cells, tab: TabSize) -> Cow<'_, str> {
    if !text.contains('\t') {
        return Cow::Borrowed(text);
    }
    let mut drawn = String::with_capacity(text.len() + tab.columns());
    let mut at = start;
    for ch in text.chars() {
        let next = advance(at, ch, tab);
        if ch == '\t' {
            drawn.extend(std::iter::repeat_n(' ', next.0 - at.0));
        } else {
            drawn.push(ch);
        }
        at = next;
    }
    Cow::Owned(drawn)
}

/// One line of the editor as it is drawn: its characters, and the tab stops
/// that decide how wide each TAB among them is.
///
/// The caret, the selection and the diagnostics underline all turn the
/// editor's character columns into screen cells against the drawn text; one
/// value keeps the three from disagreeing about a TAB.
#[derive(Clone, Copy)]
pub(super) struct EditorLine<'a> {
    pub(super) text: &'a str,
    pub(super) tab: TabSize,
}

impl EditorLine<'_> {
    /// Cells occupied by the first `column` characters.
    pub(super) fn cells_before(self, column: CharColumn) -> Cells {
        cells_before(self.text, column, self.tab)
    }

    /// The cells a window that starts `skip` cells in actually drops — see
    /// [`skip_cells`] for why that can differ from `skip`.
    pub(super) fn dropped(self, skip: Cells) -> Cells {
        skip_cells(&expand_tabs(self.text, Cells::ZERO, self.tab), skip).1
    }

    /// The width of the whole line.
    pub(super) fn width(self) -> Cells {
        Cells::of(&expand_tabs(self.text, Cells::ZERO, self.tab))
    }
}

/// The cell after `ch` when it is drawn at `at`: a TAB runs to the next stop.
fn advance(at: Cells, ch: char, tab: TabSize) -> Cells {
    if ch == '\t' {
        let columns = tab.columns();
        Cells((at.0 / columns + 1) * columns)
    } else {
        at + Cells::of_char(ch)
    }
}

/// First `width` cells of `text`, marked with a trailing ellipsis when cut.
pub(super) fn head_cells(text: &str, width: Cells) -> String {
    if Cells::of(text) <= width {
        return text.to_string();
    }
    if width.is_zero() {
        return String::new();
    }
    let (mut cut, _) = take_cells(text, width.saturating_sub(Cells(1)));
    cut.push('…');
    cut
}

/// `text` cut to `width` cells with a trailing ellipsis, then padded with
/// blanks to exactly `width` — [`head_cells`] and [`pad_cells`] composed.
///
/// One function rather than the pair at each call site: a column that cuts
/// without padding drifts out of alignment whenever a double-width glyph is
/// dropped whole and the cut comes up one cell short.
pub fn fit_name(text: &str, width: Cells) -> String {
    pad_cells(&head_cells(text, width), width)
}

/// Last `width` cells of `text`, marked with a leading ellipsis when cut.
/// Paths are cut at the head: the tail names the current directory.
pub(super) fn tail_cells(text: &str, width: Cells) -> String {
    if Cells::of(text) <= width {
        return text.to_string();
    }
    if width.is_zero() {
        return String::new();
    }
    // Walk backwards so the tail — the part that identifies the directory —
    // is what survives, keeping only as much as fits beside the ellipsis.
    let mut kept: Vec<char> = Vec::new();
    let mut used = Cells(1); // the leading '…'
    for ch in text.chars().rev() {
        let ch_cells = Cells::of_char(ch);
        if used + ch_cells > width {
            break;
        }
        kept.push(ch);
        used = used + ch_cells;
    }
    let mut cut = String::from("…");
    cut.extend(kept.into_iter().rev());
    cut
}

/// A message quoting `path` between `before` and `after`, in `width` cells.
///
/// The path gives way first, cut from its head as [`tail_cells`] cuts, since
/// what identifies the file is at its tail and the reason for the message is
/// in `after`. It keeps at least its last component — the file the message is
/// about — and only then is the line cut at its end.
pub(super) fn fit_quoting_path(before: &str, path: &str, after: &str, width: Cells) -> String {
    let words = Cells::of(before) + Cells::of(after);
    let room = width.saturating_sub(words).max(last_component_cells(path));
    head_cells(&format!("{before}{}{after}", tail_cells(path, room)), width)
}

/// The cells `path` keeps cut to its last component: the ellipsis, the
/// separator and the name — or the whole path, when that is shorter.
fn last_component_cells(path: &str) -> Cells {
    let name = std::path::Path::new(path)
        .file_name()
        .map_or(Cells::ZERO, |name| Cells::of(&name.to_string_lossy()));
    Cells::of(path).min(name + Cells(2))
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
