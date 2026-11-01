//! Purpose: Remote keys retain terminal control and navigation semantics.

use super::*;

#[test]
fn key_sequences_cover_text_control_navigation_and_function_keys() {
    let cases = [
        (KeyCode::Enter, KeyModifiers::NONE, "\r"),
        (KeyCode::Esc, KeyModifiers::NONE, "\u{1b}"),
        (KeyCode::Backspace, KeyModifiers::NONE, "\u{7f}"),
        (KeyCode::Tab, KeyModifiers::NONE, "\t"),
        (KeyCode::Char('c'), KeyModifiers::CONTROL, "\u{3}"),
        (KeyCode::Char('я'), KeyModifiers::ALT, "\u{1b}я"),
        (KeyCode::Up, KeyModifiers::NONE, "\u{1b}[A"),
        (KeyCode::Left, KeyModifiers::CONTROL, "\u{1b}[1;5D"),
        (KeyCode::F(1), KeyModifiers::NONE, "\u{1b}OP"),
        (KeyCode::F(12), KeyModifiers::SHIFT, "\u{1b}[24;2~"),
    ];
    for (code, modifiers, expected) in cases {
        assert_eq!(encode(KeyEvent::new(code, modifiers)).unwrap(), expected);
    }
}

#[test]
fn releases_are_empty_and_unsupported_keys_report_an_error() {
    let mut event = KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE);
    event.kind = KeyEventKind::Release;
    assert_eq!(encode(event).unwrap(), "");
    assert!(encode(KeyEvent::new(KeyCode::F(30), KeyModifiers::NONE)).is_err());
}

#[test]
fn russian_control_c_aliases_cancel_without_changing_plain_text() {
    for character in ['с', 'С'] {
        assert_eq!(
            encode(KeyEvent::new(
                KeyCode::Char(character),
                KeyModifiers::CONTROL
            ))
            .unwrap(),
            "\u{3}"
        );
        assert_eq!(
            encode(KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE)).unwrap(),
            character.to_string()
        );
    }
}
