//! Character-based stream selection and replacement, independent of input and I/O.

use std::ops::Range;

use crate::search::Caret;

use super::Editor;

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

    /// Remove a nonempty selection and leave the caret at its beginning.
    pub fn delete_selection(&mut self) -> bool {
        if self.selection == EditorSelection::All {
            return self.delete_all();
        }
        let range = self.selection_range();
        self.selection = EditorSelection::None;
        let Some(range) = range else {
            return false;
        };
        let first = range.start.line.get();
        let last = range.end.line.get();
        let suffix = self
            .lines
            .get(last)
            .and_then(|line| line.get(byte_at(line, range.end.column.chars())..))
            .unwrap_or_default()
            .to_owned();
        let Some(line) = self.lines.get_mut(first) else {
            return false;
        };
        line.truncate(byte_at(line, range.start.column.chars()));
        line.push_str(&suffix);
        self.lines.drain(first + 1..=last);
        self.cursor_line = first;
        self.cursor_col = range.start.column.chars();
        self.mark_modified();
        true
    }

    fn delete_all(&mut self) -> bool {
        let changed = self.lines.iter().any(|line| !line.is_empty())
            || self.lines.len() > 1
            || self.format.trailing_newline;
        self.lines = vec![String::new()];
        self.cursor_line = 0;
        self.cursor_col = 0;
        self.selection = EditorSelection::None;
        self.format.trailing_newline = false;
        if changed {
            self.mark_modified();
        }
        changed
    }

    /// Insert clipboard text, replacing any selection and normalizing line ends.
    pub fn insert_text(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        self.delete_selection();
        self.ensure_line();
        let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
        let at = self.byte_col(self.cursor_col);
        let Some(line) = self.line_mut() else {
            return;
        };
        let suffix = line.split_off(at);
        let mut parts = normalized.split('\n');
        if let Some(first) = parts.next() {
            line.push_str(first);
            self.cursor_col += first.chars().count();
        }
        let rest: Vec<_> = parts.map(str::to_owned).collect();
        if let Some(last) = rest.last() {
            let after = self.cursor_line + 1;
            self.cursor_line += rest.len();
            self.cursor_col = last.chars().count();
            self.lines.splice(after..after, rest);
        }
        if let Some(line) = self.line_mut() {
            line.push_str(&suffix);
        }
        self.mark_modified();
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
