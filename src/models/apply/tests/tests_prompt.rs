//! Purpose: unit tests for the apply prompt's field edits and buttons.

use super::*;

/// The prompt edits like every other single-field dialog.
#[test]
fn the_prompt_edits_its_field_and_moves_between_buttons() {
    let mut prompt = ApplyPrompt::default();
    assert!(!prompt.on_cancel());
    prompt.input_mut().insert_str("lsx");
    prompt.input_mut().backspace();
    assert_eq!(prompt.command(), "ls");
    assert_eq!(prompt.input().after_cursor(), "");
    prompt.toggle_button();
    assert!(prompt.on_cancel());
    prompt.toggle_button();
    assert!(!prompt.on_cancel());
    prompt.focus_cancel(true);
    assert!(prompt.on_cancel());
    prompt.focus_cancel(false);
    assert!(!prompt.on_cancel());
}
