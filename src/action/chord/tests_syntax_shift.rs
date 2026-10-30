//! Purpose: Shift's place in the text syntax of §5.4 — folded into a
//! character, written out under Ctrl, and under Alt only with a letter.

use super::tests::chord;
use super::tests_syntax::{character, error, CTRL_SHIFT};
use super::*;

#[test]
fn shift_is_part_of_a_character() {
    assert_eq!(chord("Shift+a"), character('A', Modifiers::NONE));
    assert_eq!(chord("Shift+K"), character('K', Modifiers::NONE));
    assert_eq!(chord("Shift+ж"), character('Ж', Modifiers::NONE));
    assert_eq!(chord("K"), character('K', Modifiers::NONE));
    assert_eq!(chord("k"), character('k', Modifiers::NONE));
    assert_eq!(error("Shift+1"), KeyError::ShiftedCharacter(Key::Char('1')));
    assert_eq!(error("Shift++"), KeyError::ShiftedCharacter(Key::Char('+')));
    assert_eq!(error("Shift+ß"), KeyError::ShiftedCharacter(Key::Char('ß')));
    assert_eq!(
        error("Shift+Space"),
        KeyError::ShiftedCharacter(Key::Named(NamedKey::Space))
    );
    assert_eq!(chord("Shift+F4").modifiers, Modifiers::SHIFT);
    assert_eq!(chord("Shift+Tab").modifiers, Modifiers::SHIFT);
    // Under Alt alone too: every terminal sends the symbol Shift produced.
    assert_eq!(
        error("Alt+Shift+="),
        KeyError::ShiftedCharacter(Key::Char('='))
    );
    assert_eq!(
        error("Alt+Shift+Space"),
        KeyError::ShiftedCharacter(Key::Named(NamedKey::Space))
    );
    assert_eq!(
        chord("Alt+Shift+F4").modifiers,
        Modifiers::ALT.with(Modifier::Shift)
    );
}

#[test]
fn letters_under_ctrl_or_alt_ignore_case_and_write_shift_out() {
    assert_eq!(chord("Ctrl+r"), character('r', Modifiers::CTRL));
    assert_eq!(chord("Ctrl+R"), chord("Ctrl+r"));
    assert_eq!(chord("Ctrl+Shift+R"), character('r', CTRL_SHIFT));
    assert_eq!(chord("Ctrl+Shift+r"), chord("Ctrl+Shift+R"));
    assert_ne!(chord("Ctrl+Shift+R"), chord("Ctrl+R"));
    assert_eq!(
        chord("Alt+Shift+k"),
        character('k', Modifiers::ALT.with(Modifier::Shift))
    );
    assert_eq!(
        chord("Ctrl+Alt+Ж"),
        character('ж', Modifiers::CTRL.with(Modifier::Alt))
    );
    assert_eq!(chord("Ctrl+中"), character('中', Modifiers::CTRL));
}

#[test]
fn other_characters_under_ctrl_or_alt_are_taken_as_written() {
    assert_eq!(chord("Ctrl++"), character('+', Modifiers::CTRL));
    assert_eq!(chord("Ctrl+Shift+="), character('=', CTRL_SHIFT));
    assert_eq!(chord("Alt+:"), character(':', Modifiers::ALT));
    assert_eq!(chord("Ctrl+1"), character('1', Modifiers::CTRL));
    assert_eq!(chord("Ctrl+Shift+Space").key, Key::Named(NamedKey::Space));
    assert_eq!(chord("Ctrl+Shift+Space").modifiers, CTRL_SHIFT);
    assert_eq!(
        chord("Ctrl+Alt+Shift+="),
        character('=', CTRL_SHIFT.with(Modifier::Alt))
    );
}
