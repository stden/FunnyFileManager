//! Purpose: Moving the editor's caret by character, line, and to either end of
//! a line; every move drops the selection.

use super::{Editor, EditorSelection};

impl Editor {
    pub fn move_left(&mut self) {
        self.selection = EditorSelection::None;
        if self.cursor_col > 0 {
            self.cursor_col -= 1;
        } else if self.cursor_line > 0 {
            self.cursor_line -= 1;
            self.cursor_col = self.line_char_len();
        }
    }

    pub fn move_right(&mut self) {
        self.selection = EditorSelection::None;
        if self.cursor_col < self.line_char_len() {
            self.cursor_col += 1;
        } else if self.cursor_line + 1 < self.lines.len() {
            self.cursor_line += 1;
            self.cursor_col = 0;
        }
    }

    pub const fn move_home(&mut self) {
        self.selection = EditorSelection::None;
        self.cursor_col = 0;
    }

    pub fn move_end(&mut self) {
        self.selection = EditorSelection::None;
        self.cursor_col = self.line_char_len();
    }

    pub fn move_up(&mut self, count: usize) {
        self.selection = EditorSelection::None;
        self.cursor_line = self.cursor_line.saturating_sub(count);
        self.clamp_col();
    }

    pub fn move_down(&mut self, count: usize) {
        self.selection = EditorSelection::None;
        self.cursor_line = self
            .cursor_line
            .saturating_add(count)
            .min(self.lines.len().saturating_sub(1));
        self.clamp_col();
    }
}
