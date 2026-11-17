//! Unit tests: the input prompt edits a line and hands back what was typed
//! with its purpose; the question opens on Yes and carries its purpose.

use super::{InputPrompt, InputPurpose, Question, QuestionPurpose};
use crate::models::{Confirm, SessionName};

fn name() -> SessionName {
    SessionName::new("demo@example.test").expect("name")
}

#[test]
fn an_input_prompt_opens_on_ok_with_its_suggestion() {
    let prompt = InputPrompt::new(InputPurpose::ImportNetBox, "/srv/x/demo.netbox");
    assert!(!prompt.on_cancel());
    assert_eq!(prompt.input().text(), "/srv/x/demo.netbox");
    assert_eq!(prompt.purpose(), &InputPurpose::ImportNetBox);
}

#[test]
fn the_buttons_toggle_and_the_answer_is_what_was_typed() {
    let mut prompt = InputPrompt::new(InputPurpose::DuplicateSession(name()), "copy");
    prompt.toggle_button();
    assert!(prompt.on_cancel());
    prompt.toggle_button();
    assert!(!prompt.on_cancel());
    prompt.input_mut().insert_char('!');
    assert_eq!(
        prompt.into_answer(),
        (InputPurpose::DuplicateSession(name()), "copy!".to_owned())
    );
}

#[test]
fn every_purpose_has_a_title_and_a_caption() {
    assert_eq!(
        InputPurpose::ImportNetBox.title(),
        "NetBox: import from .netbox file"
    );
    assert_eq!(InputPurpose::ImportNetBox.label(), "Read sessions from:");
    assert_eq!(
        InputPurpose::DuplicateSession(name()).title(),
        "NetBox: copy demo@example.test"
    );
    assert_eq!(
        InputPurpose::DuplicateSession(name()).label(),
        "Name of the copy:"
    );
}

#[test]
fn a_question_opens_on_yes_toggles_and_carries_its_purpose() {
    let mut question = Question::new(QuestionPurpose::DeleteSession(name()));
    assert_eq!(question.answer(), Confirm::Yes);
    assert_eq!(question.purpose().title(), "NetBox: delete session");
    assert_eq!(
        question.purpose().text(),
        "Delete the stored session demo@example.test?"
    );
    question.toggle();
    assert_eq!(question.answer(), Confirm::No);
    assert_eq!(
        question.into_purpose(),
        QuestionPurpose::DeleteSession(name())
    );
}
