//! Purpose: Typing into the editor's buffer: inserting characters and line
//! breaks, and deleting on either side of the caret.

use super::Editor;

impl Editor {
    pub fn insert_char(&mut self, ch: char) {
        self.delete_selection();
        self.ensure_line();
        let line = self.cursor_line;
        let at = self.byte_col(self.cursor_col);
        let Some(text) = self.line_mut() else {
            return;
        };
        text.insert(at, ch);
        self.cursor_col += 1;
        self.note_dirty(line, line + 1);
        self.mark_modified();
    }

    pub fn insert_newline(&mut self) {
        self.delete_selection();
        self.ensure_line();
        let line = self.cursor_line;
        let at = self.byte_col(self.cursor_col);
        let Some(text) = self.line_mut() else {
            return;
        };
        let rest = text.split_off(at);
        self.lines.insert(line + 1, rest);
        self.cursor_line += 1;
        self.cursor_col = 0;
        self.note_dirty(line, line + 2);
        self.mark_modified();
    }

    /// Delete the char before the cursor, joining with the previous line at column 0.
    pub fn backspace(&mut self) {
        if self.delete_selection() {
            return;
        }
        if self.cursor_col > 0 {
            let line = self.cursor_line;
            let at = self.byte_col(self.cursor_col - 1);
            let Some(text) = self.line_mut() else {
                return;
            };
            text.remove(at);
            self.cursor_col -= 1;
            self.note_dirty(line, line + 1);
            self.mark_modified();
        } else if self.cursor_line > 0 {
            let line = self.cursor_line - 1;
            let current = self.lines.remove(self.cursor_line);
            self.cursor_line -= 1;
            self.cursor_col = self.line_char_len();
            if let Some(previous) = self.line_mut() {
                previous.push_str(&current);
            }
            self.note_dirty(line, line + 1);
            self.mark_modified();
        }
    }

    /// Delete the char under the cursor, joining with the next line at end of line.
    pub fn delete_forward(&mut self) {
        if self.delete_selection() {
            return;
        }
        if self.cursor_col < self.line_char_len() {
            let line = self.cursor_line;
            let at = self.byte_col(self.cursor_col);
            let Some(text) = self.line_mut() else {
                return;
            };
            text.remove(at);
            self.note_dirty(line, line + 1);
            self.mark_modified();
        } else if self.cursor_line + 1 < self.lines.len() {
            let line = self.cursor_line;
            let next = self.lines.remove(line + 1);
            if let Some(text) = self.line_mut() {
                text.push_str(&next);
            }
            self.note_dirty(line, line + 1);
            self.mark_modified();
        }
    }
}
