//! Purpose: The two small dialogs the `NetBox` panels ask with — a one-line
//! input and a yes/no question — each carrying what it was asked for.
//!
//! State only: the keys live in `popup::input_prompt` / `popup::question`,
//! and what an answer does is `ops::prompts`. A purpose is an enum variant
//! with its payload, so a new question adds a variant and an arm in
//! `ops::prompts` and never touches the dialogs' keys or drawing. The field
//! is the same [`LineEditor`] the command line and the other prompts edit.

use super::{Confirm, SessionName};
use crate::lineedit::LineEditor;

/// What an [`InputPrompt`] asks for, and what the answer is for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputPurpose {
    /// The F11 import row: which `.netbox` file to read.
    ImportNetBox,
    /// F5 on the sessions panel: the copy's name.
    DuplicateSession(SessionName),
}

impl InputPurpose {
    /// The dialog's title.
    #[must_use]
    pub fn title(&self) -> String {
        match self {
            Self::ImportNetBox => "NetBox: import from .netbox file".to_owned(),
            Self::DuplicateSession(name) => format!("NetBox: copy {name}"),
        }
    }

    /// The field's caption.
    #[must_use]
    pub const fn label(&self) -> &'static str {
        match self {
            Self::ImportNetBox => "Read sessions from:",
            Self::DuplicateSession(_) => "Name of the copy:",
        }
    }
}

/// One text field, OK and Cancel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputPrompt {
    purpose: InputPurpose,
    input: LineEditor,
    /// `Yes` is OK, `No` is Cancel — the same two answers a question has.
    button: Confirm,
}

impl InputPrompt {
    /// Open on `initial` — a suggested path or name, or nothing.
    #[must_use]
    pub fn new(purpose: InputPurpose, initial: &str) -> Self {
        Self {
            purpose,
            input: LineEditor::new(initial),
            button: Confirm::Yes,
        }
    }

    #[must_use]
    pub const fn purpose(&self) -> &InputPurpose {
        &self.purpose
    }

    #[must_use]
    pub const fn input(&self) -> &LineEditor {
        &self.input
    }

    pub const fn input_mut(&mut self) -> &mut LineEditor {
        &mut self.input
    }

    #[must_use]
    pub const fn on_cancel(&self) -> bool {
        !self.button.is_yes()
    }

    pub const fn toggle_button(&mut self) {
        self.button = self.button.toggled();
    }

    /// The purpose and the typed text, for the answer's operation.
    #[must_use]
    pub fn into_answer(self) -> (InputPurpose, String) {
        (self.purpose, self.input.text().to_owned())
    }
}

/// What a [`Question`] asks, and what Yes does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuestionPurpose {
    /// F8 on the sessions panel.
    DeleteSession(SessionName),
}

impl QuestionPurpose {
    #[must_use]
    pub const fn title(&self) -> &'static str {
        match self {
            Self::DeleteSession(_) => "NetBox: delete session",
        }
    }

    /// The sentence above the buttons.
    #[must_use]
    pub fn text(&self) -> String {
        match self {
            Self::DeleteSession(name) => format!("Delete the stored session {name}?"),
        }
    }
}

/// A yes/no dialog with what Yes means.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Question {
    purpose: QuestionPurpose,
    answer: Confirm,
}

impl Question {
    /// Opens on Yes, as Far's confirmations do.
    #[must_use]
    pub const fn new(purpose: QuestionPurpose) -> Self {
        Self {
            purpose,
            answer: Confirm::Yes,
        }
    }

    #[must_use]
    pub const fn purpose(&self) -> &QuestionPurpose {
        &self.purpose
    }

    #[must_use]
    pub const fn answer(&self) -> Confirm {
        self.answer
    }

    pub const fn toggle(&mut self) {
        self.answer = self.answer.toggled();
    }

    #[must_use]
    pub fn into_purpose(self) -> QuestionPurpose {
        self.purpose
    }
}

#[cfg(test)]
mod tests;
