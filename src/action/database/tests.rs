//! Purpose: Pin the explorer's default keys: each key resolves to exactly one
//! command, and an unbound key resolves to none.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::{resolve, DatabaseCommand};
use crate::keymap::Keymap;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

#[test]
fn the_tree_and_view_keys_resolve_to_their_commands() {
    let bindings = &Keymap::default().database_explorer;
    let expected = [
        (KeyCode::Up, DatabaseCommand::Up),
        (KeyCode::Down, DatabaseCommand::Down),
        (KeyCode::Right, DatabaseCommand::Expand),
        (KeyCode::Left, DatabaseCommand::Collapse),
        (KeyCode::Enter, DatabaseCommand::Open),
        (KeyCode::Tab, DatabaseCommand::Columns),
        (KeyCode::Char('c'), DatabaseCommand::Columns),
        (KeyCode::PageUp, DatabaseCommand::PageUp),
        (KeyCode::PageDown, DatabaseCommand::PageDown),
        (KeyCode::F(5), DatabaseCommand::Reload),
        (KeyCode::Esc, DatabaseCommand::Back),
        (KeyCode::F(10), DatabaseCommand::Close),
    ];
    for (code, command) in expected {
        assert_eq!(resolve(bindings, key(code)), Some(command), "{code:?}");
    }
}

#[test]
fn an_unbound_key_resolves_to_nothing() {
    assert_eq!(
        resolve(
            &Keymap::default().database_explorer,
            key(KeyCode::Char('z'))
        ),
        None
    );
}
