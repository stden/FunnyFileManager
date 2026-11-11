//! Purpose: Key names from the feature files, read as crossterm key events.

use super::*;

/// The modifier a `Ctrl+`, `Shift+` or `Alt+` prefix names.
fn modifier(prefix: &str) -> Option<KeyModifiers> {
    match prefix {
        "Ctrl" => Some(KeyModifiers::CONTROL),
        "Shift" => Some(KeyModifiers::SHIFT),
        "Alt" => Some(KeyModifiers::ALT),
        _ => None,
    }
}

/// `Ctrl+Shift+Up`: any number of modifier prefixes, then the key they apply to.
pub fn parse_key(key: &str) -> KeyEvent {
    let mut modifiers = KeyModifiers::NONE;
    let mut name = key;
    while let Some((prefix, rest)) = name.split_once('+') {
        let Some(modifier) = modifier(prefix) else {
            break;
        };
        modifiers |= modifier;
        name = rest;
    }
    KeyEvent::new(key_code(name, modifiers), modifiers)
}

#[expect(
    clippy::expect_used,
    clippy::panic,
    reason = "the BDD harness: a scenario naming a key it does not know is a test bug and fails here"
)]
fn key_code(key: &str, modifiers: KeyModifiers) -> KeyCode {
    match key {
        "Enter" => KeyCode::Enter,
        "Esc" => KeyCode::Esc,
        "Tab" if modifiers == KeyModifiers::SHIFT => KeyCode::BackTab,
        "Tab" => KeyCode::Tab,
        "Up" => KeyCode::Up,
        "Down" => KeyCode::Down,
        "Left" => KeyCode::Left,
        "Right" => KeyCode::Right,
        "Home" => KeyCode::Home,
        "End" => KeyCode::End,
        "PgUp" => KeyCode::PageUp,
        "PgDn" => KeyCode::PageDown,
        "Ins" => KeyCode::Insert,
        "Delete" | "Del" => KeyCode::Delete,
        "Backspace" => KeyCode::Backspace,
        "Space" => KeyCode::Char(' '),
        value => match value.strip_prefix('F') {
            Some(number) if !number.is_empty() => KeyCode::F(number.parse().expect("function key")),
            _ if value.chars().count() == 1 => KeyCode::Char(
                value
                    .to_ascii_lowercase()
                    .chars()
                    .next()
                    .expect("character"),
            ),
            _ => panic!("unknown key {key}"),
        },
    }
}
