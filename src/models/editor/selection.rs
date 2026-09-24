//! Character-based stream selection and replacement, independent of input and I/O.

use std::ops::Range;

use crate::search::Caret;

use super::Editor;

mod replace;

/// Only the anchor is stored: the editor's caret is the active endpoint.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum EditorSelection {
    #[default]
    None,
    All,
    Anchored(Caret),
}

/// Whether cursor motion begins or extends a stream selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionMode {
    Move,
    Extend,
}

/// Editor movement expressed without terminal key codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorMotion {
    Left,
    Right,
    Up(usize),
    Down(usize),
    Home,
    End,
}

impl Editor {
    /// Text belongs to the document only while no editor subdialog owns input.
    pub(crate) fn accepts_text(&self) -> bool {
        self.state.exit_choice().is_none()
            && self.state.overwrite_prompt().is_none()
            && self.search.prompt.is_none()
            && self.search.not_found.is_none()
            && self.encoding_choice.is_none()
            && self.lsp.popup.is_none()
    }

    /// Select the complete document, including its final newline when present.
    pub fn select_all(&mut self) {
        self.cursor_line = self.lines.len().saturating_sub(1);
        self.cursor_col = self.lines.last().map_or(0, |line| line.chars().count());
        self.selection = EditorSelection::All;
    }

    /// Move the active caret, keeping the original anchor for Shift movement.
    pub fn move_cursor(&mut self, motion: EditorMotion, mode: SelectionMode) {
        let anchor = match self.selection {
            EditorSelection::None => self.caret(),
            EditorSelection::All => Caret::new(0, 0),
            EditorSelection::Anchored(anchor) => anchor,
        };
        match motion {
            EditorMotion::Left => self.move_left(),
            EditorMotion::Right => self.move_right(),
            EditorMotion::Up(count) => self.move_up(count),
            EditorMotion::Down(count) => self.move_down(count),
            EditorMotion::Home => self.move_home(),
            EditorMotion::End => self.move_end(),
        }
        if mode == SelectionMode::Extend {
            self.selection = EditorSelection::Anchored(anchor);
        }
    }

    /// The nonempty selection in document order, regardless of direction.
    #[must_use]
    pub fn selection_range(&self) -> Option<Range<Caret>> {
        let anchor = match self.selection {
            EditorSelection::None => return None,
            EditorSelection::All => Caret::new(0, 0),
            EditorSelection::Anchored(anchor) => anchor,
        };
        if self.lines.is_empty() {
            return None;
        }
        let anchor = self.clamp_position(anchor);
        let active = self.clamp_position(self.caret());
        (anchor != active).then(|| anchor.min(active)..anchor.max(active))
    }

    /// Copy the selected characters with the buffer's original line endings.
    #[must_use]
    pub fn selected_text(&self) -> Option<String> {
        if self.selection == EditorSelection::All {
            let content = self.content();
            return (!content.is_empty()).then_some(content);
        }
        self.selected_range_text()
    }

    fn selected_range_text(&self) -> Option<String> {
        let range = self.selection_range()?;
        let first = range.start.line.get();
        let last = range.end.line.get();
        let mut parts = Vec::with_capacity(last - first + 1);
        for (index, line) in self.lines.iter().enumerate().take(last + 1).skip(first) {
            let start = if index == first {
                byte_at(line, range.start.column.chars())
            } else {
                0
            };
            let end = if index == last {
                byte_at(line, range.end.column.chars())
            } else {
                line.len()
            };
            parts.push(line.get(start..end).unwrap_or_default());
        }
        Some(parts.join(self.format.line_ending.as_str()))
    }

    pub(super) fn ensure_line(&mut self) {
        if self.lines.is_empty() {
            self.lines.push(String::new());
        }
        self.cursor_line = self.cursor_line.min(self.lines.len() - 1);
        self.clamp_col();
    }

    fn clamp_position(&self, caret: Caret) -> Caret {
        let line = caret.line.get().min(self.lines.len().saturating_sub(1));
        let columns = self.lines.get(line).map_or(0, |text| text.chars().count());
        Caret::new(line, caret.column.chars().min(columns))
    }
}

fn byte_at(line: &str, column: usize) -> usize {
    line.char_indices()
        .nth(column)
        .map_or(line.len(), |(at, _)| at)
}

#[cfg(test)]
mod tests;
