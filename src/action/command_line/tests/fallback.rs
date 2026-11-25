//! Purpose: Unbound text, platform-specific capitals and remapped command-line keys.

use super::*;
use crate::action::chord::Platform;
use crate::keymap::{tables, ActionName, Explicit};

fn character(ch: char, modifiers: KeyModifiers) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(ch), modifiers)
}

#[test]
fn a_key_nothing_is_bound_to_resolves_to_nothing() {
    for text in ["Alt+A", "Alt+W", "Alt+Shift+B", "F5", "Ins", "PgUp"] {
        assert_eq!(by_default(press(text)), None, "{text}");
    }
    let caps_lock = KeyEvent::new(KeyCode::CapsLock, KeyModifiers::NONE);
    assert_eq!(by_default(caps_lock), None, "a key no chord names");
}

#[test]
fn a_character_without_ctrl_or_alt_types_itself_as_the_terminal_sent_it() {
    for (event, typed) in [
        (character('x', KeyModifiers::NONE), 'x'),
        (character('X', KeyModifiers::SHIFT), 'X'),
        // CapsLock: a capital without Shift stays a capital.
        (character('X', KeyModifiers::NONE), 'X'),
        (character('ж', KeyModifiers::NONE), 'ж'),
        (character(' ', KeyModifiers::SHIFT), ' '),
        (character('x', KeyModifiers::SUPER), 'x'),
    ] {
        let insert = Edit(LineEdit::Insert(typed));
        assert_eq!(by_default(event), Some(insert), "{event:?}");
    }
    // AltGr on Windows is Ctrl and Alt: its character is not typed.
    let alt_gr = KeyModifiers::CONTROL | KeyModifiers::ALT;
    assert_eq!(by_default(character('@', alt_gr)), None);
}

/// Windows upper-cases a letter under `CapsLock`, so Ctrl+R and Ctrl+W stay
/// Ctrl+R and Ctrl+W there; on Unix the capitals are kitty's Ctrl+Shift+R
/// and Ctrl+Shift+W, other keys.
#[test]
fn a_capital_under_ctrl_is_caps_lock_on_windows_and_shift_on_unix() {
    let bindings = Keymap::default().command_line;
    let read = |platform: Platform, ch: char| {
        platform
            .chord(character(ch, KeyModifiers::CONTROL))
            .and_then(|chord| bindings.command(chord))
    };
    assert_eq!(
        read(Platform::Windows, 'R'),
        Some(LineCommand::HistorySearch)
    );
    assert_eq!(read(Platform::Unix, 'R'), None);
    assert_eq!(
        read(Platform::Windows, 'W'),
        Some(Edit(LineEdit::KillWordLeft))
    );
    assert_eq!(read(Platform::Unix, 'W'), None);
}

#[test]
fn the_keys_a_file_gives_are_the_keys_obeyed() {
    let explicit = Explicit::from([
        (ActionName::new("history_search"), vec![chord("F7")]),
        (ActionName::new("clear_line"), vec![chord("!")]),
    ]);
    let bindings = tables::COMMAND_LINE.layered(&explicit).bindings;
    let resolved = |text: &str| resolve(&bindings, press(text));

    assert_eq!(resolved("F7"), Some(LineCommand::HistorySearch));
    assert_eq!(resolved("Ctrl+R"), None, "the default left with the key");
    // A bound character runs its command before it could type itself.
    assert_eq!(resolved("!"), Some(LineCommand::CopyOrClear));
    assert_eq!(resolved("Ctrl+C"), None);
    assert_eq!(resolved("Enter"), Some(LineCommand::Submit), "untouched");
}

#[test]
fn no_key_at_all_still_types() {
    let unbound = Bindings::default();
    assert_eq!(resolve(&unbound, press("Enter")), None);
    assert_eq!(
        resolve(&unbound, press("x")),
        Some(Edit(LineEdit::Insert('x')))
    );
}
