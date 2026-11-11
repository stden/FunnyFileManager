//! Purpose: What the answer to the generic input and question dialogs does.
//!
//! The dialogs know nothing about sessions; this is the one match from a
//! purpose to its operation, so a feature that needs a line of text or a
//! yes/no adds a purpose and an arm here and owns nothing in `popup` or `ui`.

use crate::models::{App, InputPurpose, QuestionPurpose};

/// OK in the input dialog, once it has closed.
pub fn submit_input(app: &mut App, purpose: InputPurpose, typed: &str) {
    match purpose {
        InputPurpose::ImportNetBox => super::sessions::import::submit(app, typed),
        InputPurpose::DuplicateSession(name) => {
            let outcome = super::sessions::duplicate(app, &name, typed);
            super::sessions::report(app, outcome);
        }
    }
}

/// Yes in the question dialog, once it has closed.
pub fn confirm_question(app: &mut App, purpose: QuestionPurpose) {
    match purpose {
        QuestionPurpose::DeleteSession(name) => {
            let outcome = super::sessions::delete(app, &name);
            super::sessions::report(app, outcome);
        }
    }
}

#[cfg(test)]
mod tests;
