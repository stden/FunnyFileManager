//! Unit tests: F3 is the one key the task list's search bar adds.

use super::*;
use crossterm::event::KeyModifiers;

#[test]
fn f3_finds_the_next_match_whatever_modifier_is_held() {
    for modifiers in [
        KeyModifiers::NONE,
        KeyModifiers::SHIFT,
        KeyModifiers::CONTROL,
    ] {
        assert_eq!(
            resolve(KeyEvent::new(KeyCode::F(3), modifiers)),
            Some(TaskSearchCommand::FindNext),
            "{modifiers:?}"
        );
    }
}

#[test]
fn every_other_key_is_the_bars_or_the_lists() {
    for code in [
        KeyCode::Enter,
        KeyCode::Esc,
        KeyCode::Down,
        KeyCode::Char('a'),
        KeyCode::Backspace,
        KeyCode::F(4),
    ] {
        assert_eq!(
            resolve(KeyEvent::new(code, KeyModifiers::NONE)),
            None,
            "{code:?}"
        );
    }
}
