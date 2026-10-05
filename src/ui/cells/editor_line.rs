//! Purpose: The editor's text in cells: every TAB widened to the next stop,
//! and the one bridge from the editor's character columns to the screen
//! cells the caret, the selection and the diagnostics underline share.

use std::borrow::Cow;

use crate::models::{CharColumn, TabSize};

use super::{skip_cells, Cells};

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
pub(in crate::ui) fn expand_tabs(text: &str, start: Cells, tab: TabSize) -> Cow<'_, str> {
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
pub(in crate::ui) struct EditorLine<'a> {
    pub(in crate::ui) text: &'a str,
    pub(in crate::ui) tab: TabSize,
}

impl EditorLine<'_> {
    /// Cells occupied by the first `column` characters.
    pub(in crate::ui) fn cells_before(self, column: CharColumn) -> Cells {
        cells_before(self.text, column, self.tab)
    }

    /// The cells a window that starts `skip` cells in actually drops — see
    /// [`skip_cells`] for why that can differ from `skip`.
    pub(in crate::ui) fn dropped(self, skip: Cells) -> Cells {
        skip_cells(&expand_tabs(self.text, Cells::ZERO, self.tab), skip).1
    }

    /// The width of the whole line.
    pub(in crate::ui) fn width(self) -> Cells {
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
