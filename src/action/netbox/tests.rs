//! Unit tests: a `NetBox` panel's default keys matched exactly, the keys a
//! file gives in their place, and the panel's actions: `from_action` stays
//! until step 6 of the keymap design, and this test holds it to the default
//! keys.

use crossterm::event::{KeyCode, KeyModifiers};

use super::*;
use crate::keymap::tables::NETBOX_PANEL;
use crate::keymap::{ActionName, Explicit, Keymap};
use crate::test_support::{chord, key_event as bare, key_event_with_mods as with};

/// The six list keys and the five row keys of the default keymap.
const BARE_KEYS: [(KeyCode, NetboxCommand); 11] = [
    (KeyCode::Up, NetboxCommand::Move(ListMotion::Up)),
    (KeyCode::Down, NetboxCommand::Move(ListMotion::Down)),
    (KeyCode::Home, NetboxCommand::Move(ListMotion::Home)),
    (KeyCode::End, NetboxCommand::Move(ListMotion::End)),
    (KeyCode::PageUp, NetboxCommand::Move(ListMotion::PageUp)),
    (KeyCode::PageDown, NetboxCommand::Move(ListMotion::PageDown)),
    (KeyCode::Enter, NetboxCommand::Row(RowCommand::Open)),
    (KeyCode::F(3), NetboxCommand::Row(RowCommand::ViewSession)),
    (KeyCode::F(4), NetboxCommand::Row(RowCommand::EditSession)),
    (KeyCode::F(5), NetboxCommand::Row(RowCommand::CopySession)),
    (KeyCode::F(8), NetboxCommand::Row(RowCommand::DeleteSession)),
];

fn defaults() -> Bindings<NetboxCommand> {
    Keymap::default().netbox_panel
}

/// Exact matching (design §6.2): Shift+Up is not Up, and Shift+F4 is the
/// one shifted key, a command of its own.
#[test]
fn a_default_key_takes_no_modifier_at_all() {
    let keys = defaults();
    let new_session = with(KeyCode::F(4), KeyModifiers::SHIFT);
    for (code, command) in BARE_KEYS {
        assert_eq!(resolve(&keys, bare(code)), Some(command), "{code:?}");
        for held in [
            KeyModifiers::SHIFT,
            KeyModifiers::CONTROL,
            KeyModifiers::ALT,
            KeyModifiers::SHIFT | KeyModifiers::CONTROL,
        ] {
            let key = with(code, held);
            let expected = (key == new_session).then_some(NetboxCommand::NewSession);
            assert_eq!(resolve(&keys, key), expected, "{code:?} {held:?}");
        }
    }
}

/// Super, Hyper and Meta are no part of a chord (design §6.1): a desktop
/// that passes one through does not make the key dead.
#[test]
fn a_key_with_super_hyper_or_meta_held_is_the_key() {
    let keys = defaults();
    for (code, command) in BARE_KEYS {
        for held in [KeyModifiers::SUPER, KeyModifiers::HYPER, KeyModifiers::META] {
            let key = with(code, held);
            assert_eq!(resolve(&keys, key), Some(command), "{code:?} {held:?}");
        }
    }
}

#[test]
fn the_keys_the_panels_do_not_own_resolve_to_nothing() {
    let keys = defaults();
    for event in [
        bare(KeyCode::Tab),
        bare(KeyCode::F(7)),
        bare(KeyCode::F(10)),
        bare(KeyCode::Insert),
        bare(KeyCode::Char(':')),
        bare(KeyCode::Char('j')),
        bare(KeyCode::CapsLock),
        with(KeyCode::F(1), KeyModifiers::ALT),
        with(KeyCode::Char('o'), KeyModifiers::CONTROL),
    ] {
        assert_eq!(resolve(&keys, event), None, "{event:?}");
    }
}

/// The resolver has no table of its own: a key the file gives an action is
/// the panel's, the default it replaced is not, and no keys own nothing.
#[test]
fn the_keys_are_the_ones_it_is_handed() {
    let file = Explicit::from([
        (ActionName::new("view_session"), vec![chord("Ctrl+F3")]),
        (
            ActionName::new("cursor_down"),
            vec![chord("j"), chord("Down")],
        ),
        (ActionName::new("new_session"), vec![]),
    ]);
    let keys = NETBOX_PANEL.layered(&file).bindings;
    let view = NetboxCommand::Row(RowCommand::ViewSession);
    let down = NetboxCommand::Move(ListMotion::Down);
    assert_eq!(resolve(&keys, chord("Ctrl+F3").to_event()), Some(view));
    assert_eq!(resolve(&keys, bare(KeyCode::F(3))), None);
    assert_eq!(resolve(&keys, bare(KeyCode::Char('j'))), Some(down));
    assert_eq!(resolve(&keys, bare(KeyCode::Down)), Some(down));
    assert_eq!(resolve(&keys, chord("Shift+F4").to_event()), None);
    let edit = NetboxCommand::Row(RowCommand::EditSession);
    assert_eq!(resolve(&keys, bare(KeyCode::F(4))), Some(edit));

    assert_eq!(resolve(&Bindings::default(), bare(KeyCode::Enter)), None);
}

/// A panels action that reaches the panel without a key — a menu row, a
/// mouse gesture — reads as the command its key would.
#[test]
fn a_key_less_action_reads_as_the_panels_command() {
    for (action, command) in [
        (Action::NavUp, NetboxCommand::Move(ListMotion::Up)),
        (Action::NavDown, NetboxCommand::Move(ListMotion::Down)),
        (Action::First, NetboxCommand::Move(ListMotion::Home)),
        (Action::Last, NetboxCommand::Move(ListMotion::End)),
        (Action::PageUp, NetboxCommand::Move(ListMotion::PageUp)),
        (Action::PageDown, NetboxCommand::Move(ListMotion::PageDown)),
        (Action::Open, NetboxCommand::Row(RowCommand::Open)),
        (Action::View, NetboxCommand::Row(RowCommand::ViewSession)),
        (Action::Edit, NetboxCommand::Row(RowCommand::EditSession)),
        (Action::Copy, NetboxCommand::Row(RowCommand::CopySession)),
        (
            Action::DeleteConfirm,
            NetboxCommand::Row(RowCommand::DeleteSession),
        ),
        (Action::NewFile, NetboxCommand::NewSession),
    ] {
        assert_eq!(
            NetboxCommand::from_action(action),
            Some(command),
            "{action:?}"
        );
    }
    for action in [Action::Move, Action::Mkdir, Action::SwitchPanel] {
        assert_eq!(NetboxCommand::from_action(action), None, "{action:?}");
    }
}

/// `from_action` reads a panels action as the default keys read the key
/// that action is printed with: a menu row, the wheel and the double click
/// do over a `NetBox` panel what their key does there.
#[test]
fn a_key_less_source_reads_as_its_default_key_does() {
    use crate::action::tests::keyed_sources::keyed_sources;
    let keys = defaults();
    for source in keyed_sources() {
        assert_eq!(
            NetboxCommand::from_action(source.action),
            resolve(&keys, source.key),
            "{}",
            source.shortcut
        );
    }
}
