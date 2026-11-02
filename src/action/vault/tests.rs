//! Unit tests: the vault's keys are its bindings, matched exactly; a
//! character no binding names is typed; every other key is nothing.

use super::*;
use crate::action::chord::{Modifier, Modifiers};
use crate::action::KeyChord;
use crate::keymap::{tables, ActionName, Explicit, Keymap};
use crate::test_support::chord;

/// Every default key of the vault (design spec, A.8) with its command.
const DEFAULTS: [(&str, VaultCommand); 18] = [
    ("Up", VaultCommand::Move(ListMotion::Up)),
    ("Down", VaultCommand::Move(ListMotion::Down)),
    ("PgUp", VaultCommand::Move(ListMotion::PageUp)),
    ("PgDn", VaultCommand::Move(ListMotion::PageDown)),
    ("Home", VaultCommand::Move(ListMotion::Home)),
    ("End", VaultCommand::Move(ListMotion::End)),
    ("Enter", VaultCommand::Activate),
    ("Ctrl+U", VaultCommand::CopyUsername),
    ("Ctrl+R", VaultCommand::Reveal),
    ("Ctrl+Z", VaultCommand::Undo),
    ("F4", VaultCommand::Edit),
    ("F2", VaultCommand::Save),
    ("F6", VaultCommand::ToggleView),
    ("F8", VaultCommand::Delete),
    ("Backspace", VaultCommand::Backspace),
    ("Ctrl+L", VaultCommand::Lock),
    ("Ctrl+C", VaultCommand::LockAndQuit),
    ("Esc", VaultCommand::Close),
];

fn defaults() -> Bindings<VaultCommand> {
    Keymap::default().vault
}

/// The vault's keys when `ffm.yml` gives `action` the key `text`.
fn rebound(action: &'static str, text: &str) -> Bindings<VaultCommand> {
    let explicit = Explicit::from([(ActionName::new(action), vec![chord(text)])]);
    tables::VAULT.layered(&explicit).bindings
}

fn press(bindings: &Bindings<VaultCommand>, text: &str) -> Option<VaultCommand> {
    resolve(bindings, chord(text).to_event())
}

fn raw(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
    KeyEvent::new(code, modifiers)
}

/// `held` and every set of modifiers that adds to it.
fn with_more_held(held: Modifiers) -> Vec<Modifiers> {
    let add = |sets: Vec<Modifiers>, modifier| {
        let more = sets.iter().map(|set: &Modifiers| set.with(modifier));
        sets.iter().copied().chain(more).collect()
    };
    let mut sets = Modifier::ALL.into_iter().fold(vec![held], add);
    sets.sort_unstable();
    sets.dedup();
    sets.retain(|&set| set != held);
    sets
}

#[test]
fn every_default_key_resolves_to_its_command() {
    let bindings = defaults();
    for (text, command) in DEFAULTS {
        assert_eq!(press(&bindings, text), Some(command), "{text}");
    }
    assert_eq!(bindings.chords().count(), DEFAULTS.len());
}

/// Exact matching: Shift+F8 deletes nothing and Ctrl+Shift+R reveals
/// nothing, where the old resolver took either for the bare key.
#[test]
fn a_default_key_with_another_modifier_held_resolves_to_nothing() {
    let bindings = defaults();
    for (text, _) in DEFAULTS {
        let KeyChord { key, modifiers } = chord(text);
        let more = with_more_held(modifiers);
        assert_eq!(more.len(), if modifiers.is_empty() { 7 } else { 3 });
        for modifiers in more {
            let pressed = KeyChord { key, modifiers };
            assert_eq!(resolve(&bindings, pressed.to_event()), None, "{pressed}");
        }
    }
}

#[test]
fn a_character_without_ctrl_or_alt_is_typed_as_the_terminal_sent_it() {
    let bindings = defaults();
    for (character, modifiers) in [
        ('a', KeyModifiers::NONE),
        ('A', KeyModifiers::SHIFT),
        // CapsLock: a capital without Shift.
        ('A', KeyModifiers::NONE),
        (' ', KeyModifiers::NONE),
        (' ', KeyModifiers::SHIFT),
        ('ж', KeyModifiers::NONE),
    ] {
        assert_eq!(
            resolve(&bindings, raw(KeyCode::Char(character), modifiers)),
            Some(VaultCommand::Type(character)),
            "{character:?} with {modifiers:?}"
        );
    }
}

#[test]
fn a_character_with_ctrl_or_alt_that_nothing_binds_resolves_to_nothing() {
    let bindings = defaults();
    for text in ["Alt+A", "Ctrl+X", "Ctrl+Alt+A", "Alt+Shift+A", "Ctrl+Space"] {
        assert_eq!(press(&bindings, text), None, "{text}");
    }
}

#[test]
fn a_key_the_vault_does_not_use_resolves_to_nothing() {
    let bindings = defaults();
    for text in [
        "Tab",
        "Shift+Tab",
        "Left",
        "Right",
        "Ins",
        "Del",
        "F1",
        "F3",
        "F10",
    ] {
        assert_eq!(press(&bindings, text), None, "{text}");
    }
    // A key no chord names.
    let caps_lock = raw(KeyCode::CapsLock, KeyModifiers::NONE);
    assert_eq!(resolve(&bindings, caps_lock), None);
}

#[test]
fn a_key_the_file_gives_an_action_runs_it_and_its_default_key_does_not() {
    let bindings = rebound("delete", "Alt+D");
    assert_eq!(press(&bindings, "Alt+D"), Some(VaultCommand::Delete));
    assert_eq!(press(&bindings, "F8"), None);
    assert_eq!(press(&bindings, "F4"), Some(VaultCommand::Edit));
}

#[test]
fn a_key_taken_from_another_action_runs_the_one_the_file_names() {
    let bindings = rebound("edit", "F8");
    assert_eq!(press(&bindings, "F8"), Some(VaultCommand::Edit));
    assert_eq!(bindings.keys(VaultCommand::Delete).count(), 0);
}

/// An explicit binding wins over the family: the character runs its
/// command and is not typed, and its capital is still text.
#[test]
fn a_character_the_file_gives_an_action_is_not_typed() {
    let bindings = rebound("toggle_view", "t");
    assert_eq!(press(&bindings, "t"), Some(VaultCommand::ToggleView));
    assert_eq!(press(&bindings, "T"), Some(VaultCommand::Type('T')));
}

#[test]
fn without_a_binding_only_a_character_means_anything() {
    let bindings = Bindings::default();
    assert_eq!(press(&bindings, "Esc"), None);
    assert_eq!(press(&bindings, "Ctrl+L"), None);
    assert_eq!(press(&bindings, "a"), Some(VaultCommand::Type('a')));
}
