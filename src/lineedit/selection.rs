//! Purpose: The selection of a `LineEditor` — Shift with a motion, or select
//! all — and what can be done with it: read it, delete it, cut it.

use std::ops::Range;

use super::{Caret, LineEditor};

impl LineEditor {
    /// The selected text's range, start first; `None` when nothing is
    /// selected, an empty range included.
    #[must_use]
    pub fn selection(&self) -> Option<Range<Caret>> {
        let anchor = self.anchor?;
        let (start, end) = (anchor.min(self.cursor), anchor.max(self.cursor));
        (start < end).then_some(Caret(start)..Caret(end))
    }

    /// The selected text, if any.
    #[must_use]
    pub fn selected_text(&self) -> Option<&str> {
        let range = self.selection()?;
        self.text.get(range.start.byte()..range.end.byte())
    }

    /// Select the whole line, caret at its end (`Ctrl+A`).
    pub const fn select_all(&mut self) {
        self.anchor = Some(0);
        self.cursor = self.text.len();
    }

    /// Move the caret with `motion`, growing or shrinking the selection from
    /// where it started (Shift with an arrow, Home or End).
    pub fn extend(&mut self, motion: fn(&mut Self)) {
        let anchor = self.anchor.unwrap_or(self.cursor);
        motion(self);
        self.anchor = Some(anchor);
    }

    /// Remove the selected text, caret where it began; whether there was any.
    pub fn delete_selection(&mut self) -> bool {
        let Some(range) = self.selection() else {
            self.anchor = None;
            return false;
        };
        self.text
            .replace_range(range.start.byte()..range.end.byte(), "");
        self.cursor = range.start.byte();
        self.anchor = None;
        true
    }

    /// Remove the selected text and hand it back (`Ctrl+X`, `Shift+Del`).
    pub fn cut(&mut self) -> Option<String> {
        let taken = self.selected_text()?.to_owned();
        self.delete_selection();
        Some(taken)
    }
}
