//! Purpose: Key names from the feature files, read as crossterm key events.

use super::*;

pub fn parse_key(key: &str) -> KeyEvent {
    let (modifiers, key) = if let Some(key) = key.strip_prefix("Ctrl+") {
        (KeyModifiers::CONTROL, key)
    } else if let Some(key) = key.strip_prefix("Shift+") {
        (KeyModifiers::SHIFT, key)
    } else {
        (KeyModifiers::NONE, key)
    };
    KeyEvent::new(key_code(key, modifiers), modifiers)
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
        "Delete" => KeyCode::Delete,
        "Backspace" => KeyCode::Backspace,
        "Space" => KeyCode::Char(' '),
        value => match value.strip_prefix('F') {
            Some(number) => KeyCode::F(number.parse().expect("function key")),
            None if value.chars().count() == 1 => KeyCode::Char(
                value
                    .to_ascii_lowercase()
                    .chars()
                    .next()
                    .expect("character"),
            ),
            None => panic!("unknown key {key}"),
        },
    }
}
