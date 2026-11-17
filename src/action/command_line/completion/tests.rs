//! Unit tests: the completion list's default keys, exactly; the keys a
//! file gives; the keys the list hands on to the command line.

use super::*;
use crate::keymap::{tables, ActionName, Explicit, Keymap};
use crate::test_support::chord;

/// The event a terminal sends for the chord `text` writes.
fn press(text: &str) -> KeyEvent {
    chord(text).to_event()
}

fn by_default(text: &str) -> Option<CompletionCommand> {
    resolve(&Keymap::default().command_line_completion, press(text))
}

#[test]
fn every_default_key_resolves_to_its_command() {
    for (text, command) in [
        ("Down", CompletionCommand::Next),
        ("Tab", CompletionCommand::Next),
        ("Up", CompletionCommand::Previous),
        ("Enter", CompletionCommand::Accept),
        ("Esc", CompletionCommand::Close),
    ] {
        assert_eq!(by_default(text), Some(command), "{text}");
    }
    let keymap = Keymap::default();
    assert_eq!(keymap.command_line_completion.chords().count(), 5);
}

#[test]
fn a_modifier_more_is_another_key() {
    for text in ["Shift+Down", "Ctrl+Up", "Alt+Tab", "Shift+Tab", "Alt+Enter"] {
        assert_eq!(by_default(text), None, "{text}");
    }
    assert_eq!(by_default("Shift+Esc"), None);
}

#[test]
fn every_other_key_goes_on_to_the_command_line() {
    for text in ["a", "Backspace", "Left", "Right", "Home", "F3", "Ctrl+R"] {
        assert_eq!(by_default(text), None, "{text}");
    }
}

#[test]
fn the_keys_a_file_gives_are_the_keys_obeyed() {
    let explicit = Explicit::from([
        (ActionName::new("next"), vec![chord("Ctrl+N")]),
        (ActionName::new("previous"), vec![chord("Ctrl+P")]),
    ]);
    let bindings = tables::COMMAND_LINE_COMPLETION.layered(&explicit).bindings;
    let resolved = |text: &str| resolve(&bindings, press(text));

    assert_eq!(resolved("Ctrl+N"), Some(CompletionCommand::Next));
    assert_eq!(resolved("Ctrl+P"), Some(CompletionCommand::Previous));
    assert_eq!(resolved("Down"), None, "handed on to the line's history");
    assert_eq!(resolved("Tab"), None);
    assert_eq!(resolved("Enter"), Some(CompletionCommand::Accept));
}
