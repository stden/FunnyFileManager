//! Unit tests: the Language servers dialog reads its list and view keys
//! from the bindings it is handed — every default resolves, a modifier
//! makes another key, and a key the file moves runs from its new place.

use super::*;
use crossterm::event::{KeyCode, KeyModifiers};

use crate::keymap::tables::LANGUAGE_SERVERS;
use crate::keymap::{ActionName, Explicit, Keymap};
use crate::test_support::{chord, key_event, key_event_with_mods};

const SPACE: KeyCode = KeyCode::Char(' ');

const KEYS: [(KeyCode, LspListCommand); 18] = [
    (KeyCode::Up, LspListCommand::Move(Step::Up)),
    (KeyCode::Down, LspListCommand::Move(Step::Down)),
    (KeyCode::PageUp, LspListCommand::Move(Step::PageUp)),
    (KeyCode::PageDown, LspListCommand::Move(Step::PageDown)),
    (KeyCode::Home, LspListCommand::Move(Step::First)),
    (KeyCode::End, LspListCommand::Move(Step::Last)),
    (KeyCode::Enter, LspListCommand::Edit),
    (KeyCode::F(4), LspListCommand::Edit),
    (SPACE, LspListCommand::ToggleEnabled),
    (KeyCode::Insert, LspListCommand::Add),
    (KeyCode::F(7), LspListCommand::Add),
    (KeyCode::Delete, LspListCommand::Remove),
    (KeyCode::F(8), LspListCommand::Remove),
    (KeyCode::F(2), LspListCommand::Save),
    (KeyCode::F(3), LspListCommand::View),
    (KeyCode::F(5), LspListCommand::Recheck),
    (KeyCode::Esc, LspListCommand::Close),
    (KeyCode::F(10), LspListCommand::Close),
];

fn defaults() -> Bindings<LspListCommand> {
    Keymap::default().language_servers
}

#[test]
fn every_default_key_resolves_to_its_command() {
    let bindings = defaults();
    for (code, command) in &KEYS {
        assert_eq!(
            resolve(&bindings, key_event(*code)),
            Some(*command),
            "{code:?}"
        );
    }
    assert_eq!(bindings.chords().count(), KEYS.len(), "and no other key");
}

/// Shift is in the character a key types, a space included, so Shift+Space
/// is the one modified key of the dialog that is not another key.
#[test]
fn a_modified_key_is_another_key() {
    let bindings = defaults();
    let held = [
        KeyModifiers::SHIFT,
        KeyModifiers::CONTROL,
        KeyModifiers::ALT,
    ];
    for (code, _) in &KEYS {
        for modifiers in held {
            if (*code, modifiers) == (SPACE, KeyModifiers::SHIFT) {
                continue;
            }
            assert_eq!(
                resolve(&bindings, key_event_with_mods(*code, modifiers)),
                None,
                "{code:?} {modifiers:?}"
            );
        }
    }
    assert_eq!(
        resolve(&bindings, key_event_with_mods(SPACE, KeyModifiers::SHIFT)),
        Some(LspListCommand::ToggleEnabled)
    );
}

/// Super, Hyper and Meta are part of no chord: a desktop that passes one
/// through does not make the key dead.
#[test]
fn a_modifier_no_chord_names_is_not_read() {
    let bindings = defaults();
    for modifiers in [KeyModifiers::SUPER, KeyModifiers::HYPER, KeyModifiers::META] {
        assert_eq!(
            resolve(&bindings, key_event_with_mods(KeyCode::F(8), modifiers)),
            Some(LspListCommand::Remove),
            "{modifiers:?}"
        );
    }
}

#[test]
fn a_key_neither_the_list_nor_the_view_uses_resolves_to_nothing() {
    let bindings = defaults();
    for code in [
        KeyCode::Tab,
        KeyCode::BackTab,
        KeyCode::F(1),
        KeyCode::F(6),
        KeyCode::Char('x'),
        KeyCode::Backspace,
        KeyCode::CapsLock,
    ] {
        assert_eq!(resolve(&bindings, key_event(code)), None, "{code:?}");
    }
}

#[test]
fn a_key_the_file_moves_runs_its_command_from_the_new_place_only() {
    let file = Explicit::from([
        (ActionName::new("remove"), vec![chord("Ctrl+D")]),
        (ActionName::new("recheck"), Vec::new()),
    ]);
    let bindings = LANGUAGE_SERVERS.layered(&file).bindings;
    let ctrl_d = key_event_with_mods(KeyCode::Char('d'), KeyModifiers::CONTROL);
    assert_eq!(resolve(&bindings, ctrl_d), Some(LspListCommand::Remove));
    for code in [KeyCode::Delete, KeyCode::F(8), KeyCode::F(5)] {
        assert_eq!(resolve(&bindings, key_event(code)), None, "{code:?}");
    }
    assert_eq!(
        resolve(&bindings, key_event(KeyCode::F(7))),
        Some(LspListCommand::Add),
        "an action the file does not name keeps its keys"
    );
}

#[test]
fn no_key_is_bound_means_no_key_resolves() {
    let bindings = Bindings::default();
    for (code, _) in &KEYS {
        assert_eq!(resolve(&bindings, key_event(*code)), None, "{code:?}");
    }
}
