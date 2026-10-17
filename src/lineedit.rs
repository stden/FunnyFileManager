//! Purpose: Pure cursor-aware line buffer backing the `:` command line.
//!
//! Pure: no `App`, no filesystem, no I/O — everything here is a function of its
//! own state, which is what makes readline behaviour cheap to test
//! exhaustively. The same reasoning that keeps [`crate::complete::matcher`]
//! dependency-free applies: cursor arithmetic is where UTF-8 bugs live, and
//! they only stay caught if the tests can reach every operation directly.

mod kill;
mod motion;
mod selection;

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

/// A single editable line: the text, the caret, the selection, and one slot
/// of killed text.
///
/// Fields are private because the type carries an invariant the compiler cannot
/// state: `cursor` is a byte index into `text` that always sits on a `char`
/// boundary and never exceeds `text.len()`. A public field would let any caller
/// park the caret mid-UTF-8, and the next insert would panic slicing there.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LineEditor {
    text: String,
    cursor: usize,
    /// Where a selection started, when there is one: the selection runs from
    /// here to the caret, either way round. On a `char` boundary, like the
    /// caret, and cleared by every edit that does not keep it.
    anchor: Option<usize>,
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
            anchor: None,
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
        self.halves().0
    }

    /// The text from the caret onwards, preserved across a completion.
    #[must_use]
    pub fn after_cursor(&self) -> &str {
        self.halves().1
    }

    /// The text split at the caret. The caret always sits on a `char` boundary
    /// within the text (see the struct), so the fallback — everything before
    /// the caret, nothing after it — is never taken.
    fn halves(&self) -> (&str, &str) {
        self.text
            .split_at_checked(self.cursor)
            .unwrap_or((&self.text, ""))
    }

    /// Replace the whole line, caret to the end (history recall, ghost accept,
    /// reverse-search accept).
    pub fn set_text(&mut self, text: &str) {
        self.anchor = None;
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
        self.anchor = None;
        self.text.clear();
        self.text.push_str(head);
        self.text.push_str(tail);
        self.cursor = head.len();
    }

    /// Empty the line. The kill slot survives, so `Ctrl+C` then `Ctrl+Y` still
    /// pastes what was killed before — as in bash.
    pub fn clear(&mut self) {
        self.anchor = None;
        self.text.clear();
        self.cursor = 0;
    }

    // ─── editing ──────────────────────────────────────────────────────────

    /// Type `ch` at the caret, over the selection if there is one.
    pub fn insert_char(&mut self, ch: char) {
        self.delete_selection();
        self.text.insert(self.cursor, ch);
        self.cursor += ch.len_utf8();
    }

    /// Insert `text` at the caret, over the selection if there is one.
    pub fn insert_str(&mut self, text: &str) {
        self.delete_selection();
        self.text.insert_str(self.cursor, text);
        self.cursor += text.len();
    }

    /// Delete the selection, or else the character before the caret. A no-op
    /// at the start of the line.
    pub fn backspace(&mut self) {
        if self.delete_selection() {
            return;
        }
        let start = self.prev_boundary();
        if start == self.cursor {
            return;
        }
        self.text.replace_range(start..self.cursor, "");
        self.cursor = start;
    }

    /// Delete the selection, or else the character under the caret. A no-op
    /// at the end of the line.
    pub fn delete_forward(&mut self) {
        if self.delete_selection() {
            return;
        }
        let end = self.next_boundary();
        if end == self.cursor {
            return;
        }
        self.text.replace_range(self.cursor..end, "");
    }
}

mod convert;

#[cfg(test)]
mod tests;
