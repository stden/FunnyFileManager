//! Unit tests: the reverse search's default keys, exactly; the keys a file
//! gives; the characters that extend the query; the keys it swallows.

use super::*;
use crate::keymap::{tables, ActionName, Explicit, Keymap};
use crate::test_support::chord;
use crossterm::event::{KeyCode, KeyModifiers};

/// The event a terminal sends for the chord `text` writes.
fn press(text: &str) -> KeyEvent {
    chord(text).to_event()
}

fn by_default(key: KeyEvent) -> Option<LineSearchCommand> {
    resolve(&Keymap::default().command_line_search, key)
}

#[test]
fn every_default_key_resolves_to_its_command() {
    for (text, command) in [
        ("Ctrl+R", LineSearchCommand::SearchOlder),
        ("Enter", LineSearchCommand::Accept),
        ("Esc", LineSearchCommand::Cancel),
        ("Ctrl+G", LineSearchCommand::Cancel),
        ("Ctrl+C", LineSearchCommand::Cancel),
        ("Backspace", LineSearchCommand::Backspace),
    ] {
        assert_eq!(by_default(press(text)), Some(command), "{text}");
    }
    assert_eq!(Keymap::default().command_line_search.chords().count(), 6);
}

#[test]
fn a_character_without_ctrl_or_alt_extends_the_query() {
    for (code, modifiers, typed) in [
        (KeyCode::Char('g'), KeyModifiers::NONE, 'g'),
        (KeyCode::Char(' '), KeyModifiers::NONE, ' '),
        (KeyCode::Char('G'), KeyModifiers::SHIFT, 'G'),
    ] {
        let event = KeyEvent::new(code, modifiers);
        let extend = LineSearchCommand::Extend(typed);
        assert_eq!(by_default(event), Some(extend), "{event:?}");
    }
}

#[test]
fn a_modifier_more_is_another_key() {
    for text in [
        "Shift+Enter",
        "Ctrl+Backspace",
        "Alt+Esc",
        "Ctrl+Shift+R",
        "Ctrl+Alt+G",
        "Ctrl+Alt+C",
    ] {
        assert_eq!(by_default(press(text)), None, "{text}");
    }
}

#[test]
fn a_key_the_search_does_not_use_resolves_to_nothing() {
    for text in ["Up", "Tab", "F3", "Ctrl+X", "Alt+A"] {
        assert_eq!(by_default(press(text)), None, "{text}");
    }
}

#[test]
fn the_keys_a_file_gives_are_the_keys_obeyed() {
    let explicit = Explicit::from([(ActionName::new("search_older"), vec![chord("Up")])]);
    let bindings = tables::COMMAND_LINE_SEARCH.layered(&explicit).bindings;

    assert_eq!(
        resolve(&bindings, press("Up")),
        Some(LineSearchCommand::SearchOlder)
    );
    assert_eq!(resolve(&bindings, press("Ctrl+R")), None);
    assert_eq!(
        resolve(&bindings, press("Esc")),
        Some(LineSearchCommand::Cancel)
    );
}
