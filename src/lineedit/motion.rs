//! Purpose: Caret movement over a `LineEditor` — by character, to the ends,
//! and by word — always landing on a `char` boundary.

use super::LineEditor;

impl LineEditor {
    // ─── cursor movement ──────────────────────────────────────────────────

    pub fn left(&mut self) {
        self.cursor = self.prev_boundary();
    }

    pub fn right(&mut self) {
        self.cursor = self.next_boundary();
    }

    pub const fn home(&mut self) {
        self.cursor = 0;
    }

    pub const fn end(&mut self) {
        self.cursor = self.text.len();
    }

    /// To the start of the word before the caret (`Alt+B` / `Ctrl+Left`).
    pub fn word_left(&mut self) {
        self.cursor = self.word_start();
    }

    /// Past the end of the word after the caret (`Alt+F` / `Ctrl+Right`).
    pub fn word_right(&mut self) {
        self.cursor = self.word_end();
    }

    // ─── boundaries ───────────────────────────────────────────────────────

    /// Byte index of the character before the caret, or the caret at the start.
    pub(super) fn prev_boundary(&self) -> usize {
        self.before_cursor()
            .chars()
            .next_back()
            .map_or(self.cursor, |ch| self.cursor - ch.len_utf8())
    }

    /// Byte index past the character under the caret, or the caret at the end.
    pub(super) fn next_boundary(&self) -> usize {
        self.after_cursor()
            .chars()
            .next()
            .map_or(self.cursor, |ch| self.cursor + ch.len_utf8())
    }

    /// Start of the word before the caret: skip whitespace backwards, then the
    /// run of non-whitespace. A *word* is a maximal run of non-whitespace, which
    /// is what makes `Ctrl+W` on `cd /a/b/c` take the whole path — the same
    /// thing bash's `unix-word-rubout` does, and the reason `Ctrl+W` is useful
    /// on a path in the first place.
    pub(super) fn word_start(&self) -> usize {
        let trimmed = self.before_cursor().trim_end();
        match trimmed.rfind(char::is_whitespace) {
            // `rfind` gives the byte index of the separator's first byte; the
            // word starts after that whole character.
            Some(i) => i + whitespace_len_at(trimmed, i),
            None => 0,
        }
    }

    /// End of the word after the caret: skip whitespace forwards, then the run
    /// of non-whitespace.
    fn word_end(&self) -> usize {
        let tail = self.after_cursor();
        let rest = tail.trim_start();
        let skipped = tail.len() - rest.len();
        let word = rest.find(char::is_whitespace).unwrap_or(rest.len());
        self.cursor + skipped + word
    }
}

/// Byte length of the whitespace character starting at `index` in `text`.
///
/// Whitespace is not always one byte — U+00A0 and U+3000 are both
/// `char::is_whitespace` — so advancing past a separator by `+ 1` would land
/// mid-character and panic the next slice.
fn whitespace_len_at(text: &str, index: usize) -> usize {
    text.get(index..)
        .and_then(|rest| rest.chars().next())
        .map_or(1, char::len_utf8)
}
