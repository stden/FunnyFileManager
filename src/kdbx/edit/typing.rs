//! Purpose: what a keystroke does to the entry form — typing into and deleting
//! from the focused field, moving the focus, and switching the mode.

use super::{EntryForm, FormMode};

impl EntryForm {
    // ── Mutations ───────────────────────────────────────────────────────

    /// Route one typed character into the focused field.
    pub fn push_char(&mut self, ch: char) {
        // Every field is single-line in v1 — Notes included — and a stored
        // newline would render as a second list row. Refuse the character
        // rather than save it.
        if ch == '\n' || ch == '\r' {
            return;
        }
        // A new keystroke means the user is retrying; stop showing the stale
        // save failure next to fresh input.
        self.error = None;
        self.remember_undo();
        self.dirty = true;
        match self.focused_text_mut() {
            Some(text) => text.push(ch),
            None => self.password.push(ch),
        }
    }

    /// Backspace in the focused field; a no-op when the field is empty.
    pub fn backspace(&mut self) {
        self.error = None;
        self.remember_undo();
        if self.remove_last_char() {
            self.dirty = true;
        } else {
            self.undo = None;
        }
    }

    /// Remove the focused field's last character; whether one was there —
    /// deleting from an empty field must not mark the draft dirty.
    fn remove_last_char(&mut self) -> bool {
        if let Some(text) = self.focused_text_mut() {
            return text.pop().is_some();
        }
        if self.password.is_empty() {
            return false;
        }
        self.password.pop();
        true
    }

    pub fn focus_next(&mut self) {
        self.focus = self.focus.next();
    }

    pub fn focus_prev(&mut self) {
        self.focus = self.focus.prev();
    }

    pub fn set_mode(&mut self, mode: FormMode) {
        self.mode = mode;
    }
}
