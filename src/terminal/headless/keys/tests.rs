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

#[test]
fn terminal_controls_and_navigation_use_standard_escape_sequences() {
    let cases = [
        (KeyCode::BackTab, "\x1b[Z"),
        (KeyCode::Null, "\0"),
        (KeyCode::Down, "\x1b[B"),
        (KeyCode::Right, "\x1b[C"),
        (KeyCode::Home, "\x1b[H"),
        (KeyCode::End, "\x1b[F"),
        (KeyCode::Insert, "\x1b[2~"),
        (KeyCode::Delete, "\x1b[3~"),
        (KeyCode::PageUp, "\x1b[5~"),
        (KeyCode::PageDown, "\x1b[6~"),
    ];
    for (code, expected) in cases {
        assert_eq!(
            encode(KeyEvent::new(code, KeyModifiers::NONE)).unwrap(),
            expected
        );
    }
}

#[test]
fn control_punctuation_preserves_terminal_meaning() {
    let cases = [
        (' ', "\0"),
        ('@', "\0"),
        ('z', "\x1a"),
        ('[', "\x1b"),
        ('\\', "\x1c"),
        (']', "\x1d"),
        ('^', "\x1e"),
        ('_', "\x1f"),
        ('?', "\x7f"),
    ];
    for (character, expected) in cases {
        assert_eq!(
            encode(KeyEvent::new(
                KeyCode::Char(character),
                KeyModifiers::CONTROL
            ))
            .unwrap(),
            expected
        );
    }
}

#[test]
fn combined_modifiers_preserve_terminal_meaning() {
    assert_eq!(
        encode(KeyEvent::new(
            KeyCode::Up,
            KeyModifiers::SHIFT | KeyModifiers::ALT | KeyModifiers::CONTROL
        ))
        .unwrap(),
        "\x1b[1;8A"
    );
    assert_eq!(
        encode(KeyEvent::new(KeyCode::Insert, KeyModifiers::ALT)).unwrap(),
        "\x1b[2;3~"
    );
    assert_eq!(
        encode(KeyEvent::new(
            KeyCode::Char('a'),
            KeyModifiers::ALT | KeyModifiers::CONTROL
        ))
        .unwrap(),
        "\x1b\x01"
    );
}

#[test]
fn every_function_key_retains_its_distinct_terminal_number() {
    let sequences = [
        "\x1bOP", "\x1bOQ", "\x1bOR", "\x1bOS", "\x1b[15~", "\x1b[17~", "\x1b[18~", "\x1b[19~",
        "\x1b[20~", "\x1b[21~", "\x1b[23~", "\x1b[24~",
    ];
    for (number, expected) in (1..=12).zip(sequences) {
        assert_eq!(
            encode(KeyEvent::new(KeyCode::F(number), KeyModifiers::NONE)).unwrap(),
            expected
        );
    }
    assert_eq!(
        encode(KeyEvent::new(KeyCode::F(4), KeyModifiers::CONTROL)).unwrap(),
        "\x1b[1;5S"
    );
}

#[test]
fn unsupported_keys_return_actionable_input_errors() {
    for (code, modifiers, message) in [
        (
            KeyCode::Char('é'),
            KeyModifiers::CONTROL,
            "Unsupported control key",
        ),
        (
            KeyCode::CapsLock,
            KeyModifiers::NONE,
            "Unsupported terminal key",
        ),
        (
            KeyCode::F(0),
            KeyModifiers::NONE,
            "Unsupported function key",
        ),
    ] {
        let error = encode(KeyEvent::new(code, modifiers)).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
        assert_eq!(error.to_string(), message);
    }
}
