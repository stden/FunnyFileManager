//! Purpose: Replacing the editor's selection: deleting it, and pasting text in
//! its place with the line endings normalized.

use super::{byte_at, Editor, EditorSelection};

impl Editor {
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
}
