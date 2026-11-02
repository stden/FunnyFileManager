//! Purpose: what the editor makes of a key nothing is bound to — a
//! character types itself as the terminal sent it, every other key is
//! consumed.

use super::{command, ALT, CTRL, NONE, SHIFT};
use crate::action::editor::EditorCommand;
use crossterm::event::{KeyCode, KeyModifiers, MediaKeyCode};

#[test]
fn a_character_without_ctrl_or_alt_types_itself() {
    for (typed, modifiers) in [
        ('x', NONE),
        ('界', NONE),
        ('Ж', SHIFT),
        ('+', SHIFT),
        (' ', NONE),
        (' ', SHIFT),
    ] {
        assert_eq!(
            command(KeyCode::Char(typed), modifiers),
            EditorCommand::Insert(typed),
            "{modifiers:?} {typed:?}"
        );
    }
}

/// The character is read from the event, not from the chord it names:
/// `CapsLock` sends a capital without Shift, and with Shift a small letter.
#[test]
fn caps_lock_types_the_letter_the_terminal_sent() {
    assert_eq!(
        command(KeyCode::Char('K'), NONE),
        EditorCommand::Insert('K')
    );
    assert_eq!(
        command(KeyCode::Char('k'), SHIFT),
        EditorCommand::Insert('k')
    );
}

#[test]
fn a_character_under_ctrl_or_alt_that_nothing_is_bound_to_is_consumed() {
    for modifiers in [CTRL, ALT, CTRL.union(ALT), ALT.union(SHIFT)] {
        assert_eq!(
            command(KeyCode::Char('x'), modifiers),
            EditorCommand::Ignore,
            "{modifiers:?}"
        );
    }
}

#[test]
fn a_key_no_chord_names_is_consumed() {
    for code in [
        KeyCode::Null,
        KeyCode::CapsLock,
        KeyCode::F(25),
        KeyCode::Media(MediaKeyCode::Play),
    ] {
        for modifiers in [NONE, CTRL, KeyModifiers::all()] {
            assert_eq!(command(code, modifiers), EditorCommand::Ignore, "{code:?}");
        }
    }
}
