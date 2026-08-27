//! Purpose: Pure cursor-aware line buffer backing the `:` command line.
//!
//! Pure: no `App`, no filesystem, no I/O — everything here is a function of its
//! own state, which is what makes readline behaviour cheap to test
//! exhaustively. The same reasoning that keeps [`crate::complete::matcher`]
//! dependency-free applies: cursor arithmetic is where UTF-8 bugs live, and
//! they only stay caught if the tests can reach every operation directly.

/// A byte offset into a [`LineEditor`]'s text that sits on a `char` boundary.
///
/// A newtype rather than a bare `usize` because the offset leaves the editor:
/// `ui` slices the text with it to place the caret, and a plain integer there
/// is indistinguishable from a column count, a char count, or a scroll offset —
/// three things this file's neighbours also measure in `usize`. Mixing them up
/// splits a multi-byte character and panics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Caret(usize);

impl Caret {
    /// The offset in bytes, for slicing the text it came from.
    #[must_use]
    pub const fn byte(self) -> usize {
        self.0
    }
}

/// A single editable line: the text, the caret, and one slot of killed text.
///
/// Fields are private because the type carries an invariant the compiler cannot
/// state: `cursor` is a byte index into `text` that always sits on a `char`
/// boundary and never exceeds `text.len()`. A public field would let any caller
/// park the caret mid-UTF-8, and the next insert would panic slicing there.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LineEditor {
    text: String,
    cursor: usize,
    /// Most recently killed run (`Ctrl+W` / `Ctrl+U` / `Ctrl+K`), yanked back
    /// by `Ctrl+Y`. One slot, not bash's rotating ring: `Alt+Y` rotation has no
    /// use in a file manager's command line, and a ring is state to keep
    /// consistent for no gain.
    kill: String,
}

impl LineEditor {
    /// A line holding `text`, caret at the end — where a user who just recalled
    /// or pasted a line expects to continue typing.
    #[must_use]
    pub fn new(text: &str) -> Self {
        Self {
            cursor: text.len(),
            text: text.to_owned(),
            kill: String::new(),
        }
    }

    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Caret position, always on a `char` boundary. Byte-indexed rather than
    /// char-indexed because every consumer (slicing for render, splitting for
    /// completion) needs bytes anyway.
    #[must_use]
    pub const fn cursor(&self) -> Caret {
        Caret(self.cursor)
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    /// Whether the caret sits past the last character.
    ///
    /// The ghost-text accept gesture (`Right` / `End`) is only offered here, so
    /// mid-line cursor movement and suggestion acceptance never fight over the
    /// same key — see the key matrix in `popup.rs`.
    #[must_use]
    pub const fn at_end(&self) -> bool {
        self.cursor == self.text.len()
    }

    /// The text before the caret — the part completion treats as typed.
    #[must_use]
    pub fn before_cursor(&self) -> &str {
        &self.text[..self.cursor]
    }

    /// The text from the caret onwards, preserved across a completion.
    #[must_use]
    pub fn after_cursor(&self) -> &str {
        &self.text[self.cursor..]
    }

    /// Replace the whole line, caret to the end (history recall, ghost accept,
    /// reverse-search accept).
    pub fn set_text(&mut self, text: &str) {
        self.text.clear();
        self.text.push_str(text);
        self.cursor = self.text.len();
    }

    /// Replace the line with `head` then `tail`, caret between the two.
    ///
    /// This is how a completion lands: the token before the caret is rewritten
    /// while everything after it is preserved, and the caret stays where the
    /// user was typing instead of jumping to the end of a line they did not
    /// mean to edit.
    pub fn set_split(&mut self, head: &str, tail: &str) {
        self.text.clear();
        self.text.push_str(head);
        self.text.push_str(tail);
        self.cursor = head.len();
    }

    /// Empty the line. The kill slot survives, so `Ctrl+C` then `Ctrl+Y` still
    /// pastes what was killed before — as in bash.
    pub fn clear(&mut self) {
        self.text.clear();
        self.cursor = 0;
    }

    // ─── editing ──────────────────────────────────────────────────────────

    pub fn insert_char(&mut self, ch: char) {
        self.text.insert(self.cursor, ch);
        self.cursor += ch.len_utf8();
    }

    pub fn insert_str(&mut self, text: &str) {
        self.text.insert_str(self.cursor, text);
        self.cursor += text.len();
    }

    /// Delete the character before the caret. A no-op at the start of the line.
    pub fn backspace(&mut self) {
        let start = self.prev_boundary();
        if start == self.cursor {
            return;
        }
        self.text.replace_range(start..self.cursor, "");
        self.cursor = start;
    }

    /// Delete the character under the caret. A no-op at the end of the line.
    pub fn delete_forward(&mut self) {
        let end = self.next_boundary();
        if end == self.cursor {
            return;
        }
        self.text.replace_range(self.cursor..end, "");
    }

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
        if start >= end {
            return;
        }
        self.kill = self.text[start..end].to_owned();
        self.text.replace_range(start..end, "");
    }

    // ─── boundaries ───────────────────────────────────────────────────────

    /// Byte index of the character before the caret, or the caret at the start.
    fn prev_boundary(&self) -> usize {
        self.text[..self.cursor]
            .chars()
            .next_back()
            .map_or(self.cursor, |ch| self.cursor - ch.len_utf8())
    }

    /// Byte index past the character under the caret, or the caret at the end.
    fn next_boundary(&self) -> usize {
        self.text[self.cursor..]
            .chars()
            .next()
            .map_or(self.cursor, |ch| self.cursor + ch.len_utf8())
    }

    /// Start of the word before the caret: skip whitespace backwards, then the
    /// run of non-whitespace. A *word* is a maximal run of non-whitespace, which
    /// is what makes `Ctrl+W` on `cd /a/b/c` take the whole path — the same
    /// thing bash's `unix-word-rubout` does, and the reason `Ctrl+W` is useful
    /// on a path in the first place.
    fn word_start(&self) -> usize {
        let head = &self.text[..self.cursor];
        let trimmed = head.trim_end();
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
        let tail = &self.text[self.cursor..];
        let skipped = tail.len() - tail.trim_start().len();
        let rest = &tail[skipped..];
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
    text[index..].chars().next().map_or(1, char::len_utf8)
}

impl From<&str> for LineEditor {
    fn from(text: &str) -> Self {
        Self::new(text)
    }
}

impl From<String> for LineEditor {
    fn from(text: String) -> Self {
        Self {
            cursor: text.len(),
            text,
            kill: String::new(),
        }
    }
}

#[cfg(test)]
mod tests;
