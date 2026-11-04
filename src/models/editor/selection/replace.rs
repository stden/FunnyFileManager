//! Purpose: Replacing the editor's selection: deleting it, and pasting text in
//! its place with the line endings normalized.

use std::borrow::Cow;

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
        if let Some(line) = self.lines.get_mut(first) {
            line.truncate(byte_at(line, range.start.column.chars()));
            line.push_str(&suffix);
        }
        self.lines.drain(first + 1..=last);
        self.cursor_line = first;
        self.cursor_col = range.start.column.chars();
        self.note_dirty(first, first + 1);
        self.mark_modified();
        true
    }

    fn delete_all(&mut self) -> bool {
        let has_body = self.lines.iter().any(|line| !line.is_empty()) || self.lines.len() > 1;
        let changed = has_body || self.format.trailing_newline;
        if !has_body {
            self.format.trailing_newline = false;
        }
        self.lines = vec![String::new()];
        self.cursor_line = 0;
        self.cursor_col = 0;
        self.selection = EditorSelection::None;
        if changed {
            self.note_dirty(0, 1);
            self.mark_modified();
        }
        changed
    }

    /// Insert clipboard text, replacing any selection and normalizing line ends.
    pub fn insert_text(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        let normalized = self.prepare_insert(text);
        let line_at = self.cursor_line;
        let at = self.byte_col(self.cursor_col);
        let suffix = self
            .line_mut()
            .map_or_else(String::new, |line| line.split_off(at));
        let mut parts = normalized.split('\n');
        if let (Some(first), Some(line)) = (parts.next(), self.line_mut()) {
            line.push_str(first);
            self.cursor_col += first.chars().count();
        }
        let rest: Vec<_> = parts.map(str::to_owned).collect();
        let added = rest.len();
        if let Some(last) = rest.last() {
            let after = self.cursor_line + 1;
            self.cursor_line += rest.len();
            self.cursor_col = last.chars().count();
            self.lines.splice(after..after, rest);
        }
        if let Some(line) = self.line_mut() {
            line.push_str(&suffix);
        }
        self.note_dirty(line_at, line_at + added + 1);
        self.mark_modified();
    }

    fn prepare_insert<'a>(&mut self, text: &'a str) -> Cow<'a, str> {
        let strip_trailing = self.selection == EditorSelection::All && self.format.trailing_newline;
        self.delete_selection();
        if strip_trailing {
            self.format.trailing_newline = true;
        }
        self.ensure_line();
        if !text.contains('\r') {
            let trimmed = if strip_trailing {
                text.strip_suffix('\n').unwrap_or(text)
            } else {
                text
            };
            return Cow::Borrowed(trimmed);
        }
        let mut out = String::with_capacity(text.len());
        let mut chars = text.chars().peekable();
        while let Some(ch) = chars.next() {
            if ch == '\r' {
                chars.next_if_eq(&'\n');
                out.push('\n');
            } else {
                out.push(ch);
            }
        }
        if strip_trailing && out.ends_with('\n') {
            out.pop();
        }
        Cow::Owned(out)
    }
}

#[cfg(test)]
mod tests_offbuffer;
