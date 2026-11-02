//! Purpose: Moving the editor's caret by character, line, and to either end of
//! a line; every move drops the selection.

use unicode_width::UnicodeWidthChar;

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

    pub(super) const fn move_document_start(&mut self) {
        self.selection = EditorSelection::None;
        self.cursor_line = 0;
        self.cursor_col = 0;
    }

    pub(super) fn move_document_end(&mut self) {
        self.selection = EditorSelection::None;
        self.cursor_line = self.lines.len().saturating_sub(1);
        self.cursor_col = self.line_char_len();
    }

    /// Cells occupied by the first `cursor_col` characters on the cursor line.
    ///
    /// TABs advance to the next stop; other characters contribute their
    /// Unicode display width.  Used to derive `scroll_x` in [`Self::viewport`].
    pub(super) fn cells_before_cursor(&self) -> usize {
        let line = self.lines.get(self.cursor_line).map_or("", String::as_str);
        let tab = self.tab_size.columns();
        let mut at = 0usize;
        for ch in line.chars().take(self.cursor_col) {
            if ch == '\t' {
                at = (at / tab + 1) * tab;
            } else {
                at += UnicodeWidthChar::width(ch).unwrap_or(0);
            }
        }
        at
    }

    /// The char index of the character that occupies screen cell `target_cell`
    /// on `line_index`.
    ///
    /// Walks the line the same way [`Self::cells_before_cursor`] does.  Returns
    /// the char index of the first character whose cell range contains
    /// `target_cell`, or the total char count when the click is past the end of
    /// the line.
    pub(super) fn char_col_for_cell(&self, line_index: usize, target_cell: usize) -> usize {
        let line = self.lines.get(line_index).map_or("", String::as_str);
        let tab = self.tab_size.columns();
        let mut at = 0usize;
        for (i, ch) in line.chars().enumerate() {
            let w = if ch == '\t' {
                (at / tab + 1) * tab - at
            } else {
                UnicodeWidthChar::width(ch).unwrap_or(0)
            };
            if at + w > target_cell {
                return i;
            }
            at += w;
        }
        line.chars().count()
    }

    /// The `(scroll_y, scroll_x)` the renderer would choose for the current
    /// cursor position at the given terminal size.
    ///
    /// Mirrors the stateless scroll formulas in `render_editor` so a mouse
    /// click handler can invert screen coordinates back into document
    /// coordinates without storing scroll state in the model.
    pub fn viewport(&self, terminal_height: usize, terminal_width: usize) -> (usize, usize) {
        const EDITOR_BAR_ROWS: usize = 1;
        let inner_h = terminal_height.saturating_sub(EDITOR_BAR_ROWS);
        let scroll_y = self.cursor_line.saturating_sub(inner_h / 2);
        let cursor_cell = self.cells_before_cursor();
        let scroll_x = cursor_cell.saturating_sub(terminal_width.saturating_sub(1));
        (scroll_y, scroll_x)
    }

    /// Place the cursor at the document line and screen cell that corresponds
    /// to a mouse click at the given `(line, cell)` in viewport coordinates.
    ///
    /// Clears the selection.  Clamps `line` to the last document line.
    pub fn jump_to_screen_position(&mut self, line: usize, cell: usize) {
        self.selection = EditorSelection::None;
        self.cursor_line = line.min(self.lines.len().saturating_sub(1));
        self.cursor_col = self.char_col_for_cell(self.cursor_line, cell);
    }
}
