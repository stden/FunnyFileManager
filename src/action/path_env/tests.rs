//! Unit tests: the PATH editor's keys are the chords its bindings hold —
//! exactly those, a modifier more being another key — and other chords once
//! the bindings say so.

use super::*;
use crate::keymap::{tables::PATH_EDITOR, ActionName, Explicit, Keymap};
use crate::test_support::chord;
use crossterm::event::{KeyCode, KeyModifiers};

/// Every default key of the list beside its command.
const DEFAULTS: [(&str, PathCommand); 15] = [
    ("Up", PathCommand::CursorUp),
    ("Down", PathCommand::CursorDown),
    ("Home", PathCommand::CursorHome),
    ("End", PathCommand::CursorEnd),
    ("Ctrl+Up", PathCommand::MoveEntryUp),
    ("Ctrl+Down", PathCommand::MoveEntryDown),
    ("Ins", PathCommand::Add),
    ("F7", PathCommand::Add),
    ("Enter", PathCommand::Edit),
    ("F4", PathCommand::Edit),
    ("Del", PathCommand::Remove),
    ("F8", PathCommand::Remove),
    ("F5", PathCommand::Recheck),
    ("F2", PathCommand::Save),
    ("Esc", PathCommand::Close),
];

/// What the key written `text` asks of the list under `bindings`.
fn pressed(bindings: &Bindings<PathCommand>, text: &str) -> Option<PathCommand> {
    resolve(bindings, chord(text).to_event())
}

#[test]
fn every_default_key_resolves_to_its_command() {
    let bindings = Keymap::default().path_editor;
    for (text, command) in DEFAULTS {
        assert_eq!(pressed(&bindings, text), Some(command), "{text}");
    }
    assert_eq!(bindings.chords().count(), DEFAULTS.len());
}

#[test]
fn a_modifier_more_is_another_key() {
    let bindings = Keymap::default().path_editor;
    for text in [
        "Shift+Up",
        "Alt+Down",
        "Ctrl+Home",
        "Shift+End",
        "Ctrl+Shift+Up",
        "Ctrl+Alt+Down",
        "Shift+Ins",
        "Alt+F7",
        "Alt+Enter",
        "Shift+F4",
        "Shift+Del",
        "Shift+F8",
        "Ctrl+F5",
        "Ctrl+F2",
        "Shift+Esc",
    ] {
        assert_eq!(pressed(&bindings, text), None, "{text}");
    }
}

#[test]
fn a_key_the_list_does_not_use_resolves_to_nothing() {
    let bindings = Keymap::default().path_editor;
    for text in ["Tab", "PgUp", "PgDn", "F1", "F3", "F10", "x", "Backspace"] {
        assert_eq!(pressed(&bindings, text), None, "{text}");
    }
    let no_chord = KeyEvent::new(KeyCode::CapsLock, KeyModifiers::NONE);
    assert_eq!(resolve(&bindings, no_chord), None);
}

#[test]
fn the_super_key_held_does_not_change_the_key() {
    let bindings = Keymap::default().path_editor;
    let held = KeyEvent::new(KeyCode::F(2), KeyModifiers::SUPER);
    assert_eq!(resolve(&bindings, held), Some(PathCommand::Save));
}

#[test]
fn a_key_the_file_gives_an_action_replaces_its_defaults() {
    let mut file = Explicit::new();
    file.insert(ActionName::new("save"), vec![chord("Ctrl+S")]);
    file.insert(ActionName::new("recheck"), vec![chord("F2")]);
    let bindings = PATH_EDITOR.layered(&file).bindings;
    assert_eq!(pressed(&bindings, "Ctrl+S"), Some(PathCommand::Save));
    assert_eq!(pressed(&bindings, "F2"), Some(PathCommand::Recheck));
    assert_eq!(pressed(&bindings, "F5"), None);
    assert_eq!(pressed(&bindings, "Esc"), Some(PathCommand::Close));
}

#[test]
fn no_bindings_leave_every_key_without_a_command() {
    let bindings = Bindings::default();
    for (text, _) in DEFAULTS {
        assert_eq!(pressed(&bindings, text), None, "{text}");
    }
}
