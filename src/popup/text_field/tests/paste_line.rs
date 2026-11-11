//! Purpose: A pasted block becomes one line, with the breaks turned into spaces.

use super::super::one_line;

#[test]
fn a_pasted_text_becomes_one_line() {
    assert_eq!(one_line("a\r\nb\nc\n"), "a b c");
    assert_eq!(one_line("plain"), "plain");
    assert_eq!(one_line("\n\n"), "");
}

#[test]
fn ctrl_right_moves_caret_by_word() {
    use crossterm::event::{KeyCode, KeyModifiers};
    let mut line = crate::lineedit::LineEditor::new("two words");
    line.home();
    let mut slot = crate::models::ClipboardSlot::default();
    assert!(!super::super::edit(
        &mut line,
        KeyCode::Right,
        KeyModifiers::CONTROL,
        &mut slot,
    ));
    assert!(super::super::edit(
        &mut line,
        KeyCode::Char('!'),
        KeyModifiers::NONE,
        &mut slot,
    ));
    assert_eq!(line.text(), "two! words");
}
