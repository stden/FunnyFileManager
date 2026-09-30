//! Purpose: The kill slot of a `LineEditor` — `Ctrl+W`, `Ctrl+U` and `Ctrl+K`
//! cut into it, `Ctrl+Y` pastes it back.

use super::LineEditor;

impl LineEditor {
    // ─── kill and yank ────────────────────────────────────────────────────

    /// Cut the word before the caret into the kill slot (`Ctrl+W`).
    pub fn kill_word_left(&mut self) {
        let start = self.word_start();
        self.kill_range(start, self.cursor);
        self.cursor = start;
    }

    /// Cut everything before the caret into the kill slot (`Ctrl+U`).
    pub fn kill_to_start(&mut self) {
        self.kill_range(0, self.cursor);
        self.cursor = 0;
    }

    /// Cut everything from the caret onwards into the kill slot (`Ctrl+K`).
    pub fn kill_to_end(&mut self) {
        self.kill_range(self.cursor, self.text.len());
    }

    /// Insert the killed text at the caret (`Ctrl+Y`).
    pub fn yank(&mut self) {
        if self.kill.is_empty() {
            return;
        }
        // Take the slot out to satisfy the borrow checker, then put it back:
        // bash's yank does not consume the ring, so repeated `Ctrl+Y` repeats
        // the paste.
        let killed = std::mem::take(&mut self.kill);
        self.insert_str(&killed);
        self.kill = killed;
    }

    /// Move `range` out of the line and into the kill slot. An empty range
    /// leaves the previous kill intact — `Ctrl+K` at the end of the line must
    /// not silently erase what `Ctrl+W` just captured.
    fn kill_range(&mut self, start: usize, end: usize) {
        self.anchor = None;
        let Some(killed) = self
            .text
            .get(start..end)
            .filter(|killed| !killed.is_empty())
        else {
            return;
        };
        self.kill = killed.to_owned();
        self.text.replace_range(start..end, "");
    }
}
