//! The WSL dialog reads its keys from the keymap, each with exactly its
//! modifiers; platform and package updates stay distinct commands.

use crossterm::event::{KeyCode, KeyModifiers};

use super::*;
use crate::keymap::{tables, ActionName, Explicit, Keymap};
use crate::test_support::chord;

/// The default keys of the dialog, as `ffm.yml` writes them.
const DEFAULTS: [(&str, WslCommand); 9] = [
    ("Up", WslCommand::Up),
    ("Down", WslCommand::Down),
    ("Home", WslCommand::First),
    ("End", WslCommand::Last),
    ("Enter", WslCommand::Open),
    ("F2", WslCommand::Refresh),
    ("F5", WslCommand::Update),
    ("F6", WslCommand::UpdatePackages),
    ("Esc", WslCommand::Close),
];

/// Every set of modifiers a key can be held with.
const MODIFIERS: [&str; 7] = [
    "Shift+",
    "Alt+",
    "Alt+Shift+",
    "Ctrl+",
    "Ctrl+Shift+",
    "Ctrl+Alt+",
    "Ctrl+Alt+Shift+",
];

fn defaults() -> Bindings<WslCommand> {
    Keymap::default().wsl_manager
}

/// What the key written `text` runs, pressed as a terminal sends it.
fn pressed(bindings: &Bindings<WslCommand>, text: &str) -> WslCommand {
    resolve(bindings, chord(text).to_event())
}

#[test]
fn navigation_and_operations_resolve_to_distinct_commands() {
    let keys = defaults();
    for (key, command) in DEFAULTS {
        assert_eq!(pressed(&keys, key), command, "{key}");
    }
}

#[test]
fn a_key_held_with_a_modifier_is_another_key_and_runs_nothing() {
    let keys = defaults();
    for (key, _) in DEFAULTS {
        for modifiers in MODIFIERS {
            let modified = format!("{modifiers}{key}");
            assert_eq!(pressed(&keys, &modified), WslCommand::Ignore, "{modified}");
        }
    }
}

#[test]
fn unbound_and_unbindable_keys_are_consumed_without_operations() {
    let keys = defaults();
    for code in [
        KeyCode::F(8),
        KeyCode::Char('x'),
        KeyCode::Insert,
        KeyCode::CapsLock,
    ] {
        let key = KeyEvent::new(code, KeyModifiers::NONE);
        assert_eq!(resolve(&keys, key), WslCommand::Ignore, "{code:?}");
    }
}

/// Super is no part of a chord, so a desktop that passes it through does
/// not make a key dead.
#[test]
fn a_modifier_no_chord_names_leaves_the_key_what_it_is() {
    let key = KeyEvent::new(KeyCode::F(5), KeyModifiers::SUPER);
    assert_eq!(resolve(&defaults(), key), WslCommand::Update);
}

#[test]
fn a_key_the_file_gives_an_action_replaces_its_default() {
    let file = Explicit::from([(ActionName::new("refresh"), vec![chord("Ctrl+R")])]);
    let keys = tables::WSL_MANAGER.layered(&file).bindings;
    assert_eq!(pressed(&keys, "Ctrl+R"), WslCommand::Refresh);
    assert_eq!(pressed(&keys, "F2"), WslCommand::Ignore);
    assert_eq!(pressed(&keys, "F5"), WslCommand::Update);
}

#[test]
fn a_default_key_given_to_another_action_runs_that_action() {
    let file = Explicit::from([(ActionName::new("update_packages"), vec![chord("F5")])]);
    let keys = tables::WSL_MANAGER.layered(&file).bindings;
    assert_eq!(pressed(&keys, "F5"), WslCommand::UpdatePackages);
    assert_eq!(pressed(&keys, "F6"), WslCommand::Ignore);
}

#[test]
fn without_a_binding_every_key_is_ignored() {
    let keys = Bindings::default();
    for (key, _) in DEFAULTS {
        assert_eq!(pressed(&keys, key), WslCommand::Ignore, "{key}");
    }
}
