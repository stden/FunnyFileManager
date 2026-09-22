//! Purpose: The describe-files prompt (`Ctrl+Z`): a description for the entry under the cursor.
//!
//! State only: the operations that fill it live in `ops`, and the keys that
//! move through it in `popup`.
//!
//! Far describes the selected files one after another, one dialog each, so
//! the prompt carries the names still waiting behind the one on screen. Enter
//! writes and moves on; Esc drops the rest, which is what a user who has just
//! changed their mind wants.

use crate::lineedit::LineEditor;

/// The entry being described, the text as typed, and the queue behind it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DescribePrompt {
    name: String,
    /// A line with a caret rather than a bare string: Far's field is an edit
    /// control, so Left goes back to the typo instead of to Cancel.
    text: LineEditor,
    /// Marked entries still to describe after this one, in panel order.
    pending: Vec<String>,
    /// The cursor is on Cancel rather than on OK.
    on_cancel: bool,
}

impl DescribePrompt {
    /// Open on `name`, offering `text` — its current description — so that
    /// editing a description is not retyping it. The caret starts at the end,
    /// where a user who wants to add a word expects it.
    #[must_use]
    pub fn new(name: String, text: String, pending: Vec<String>) -> Self {
        Self {
            name,
            text: LineEditor::new(&text),
            pending,
            on_cancel: false,
        }
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn text(&self) -> &str {
        self.text.text()
    }

    /// What stands to the right of the caret — the width `ui` backs the
    /// terminal cursor up by from the end of the field.
    #[must_use]
    pub fn after_caret(&self) -> &str {
        self.text.after_cursor()
    }

    #[must_use]
    pub fn pending(&self) -> &[String] {
        &self.pending
    }

    #[must_use]
    pub const fn on_cancel(&self) -> bool {
        self.on_cancel
    }

    /// A click names the button before Enter, so an old keyboard focus on
    /// Cancel cannot make a click on OK cancel.
    pub const fn focus_cancel(&mut self, cancel: bool) {
        self.on_cancel = cancel;
    }

    pub const fn toggle_button(&mut self) {
        self.on_cancel = !self.on_cancel;
    }

    pub fn push(&mut self, ch: char) {
        self.text.insert_char(ch);
    }

    pub fn backspace(&mut self) {
        self.text.backspace();
    }

    pub fn delete_forward(&mut self) {
        self.text.delete_forward();
    }

    pub fn left(&mut self) {
        self.text.left();
    }

    pub fn right(&mut self) {
        self.text.right();
    }

    pub const fn home(&mut self) {
        self.text.home();
    }

    pub const fn end(&mut self) {
        self.text.end();
    }
}
