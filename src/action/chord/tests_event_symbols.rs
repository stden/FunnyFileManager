//! Purpose: `from_event` for a character that is not a letter under Ctrl or
//! Alt — the §2.2 rows for `Ctrl+Shift+=`, `AltGr` and NUL, and the symbols
//! Shift produces under Alt, which every terminal sends as the symbol.

use crossterm::event::KeyCode;

use super::tests_event::{check, Row, ALT, ALT_SHIFT, CTRL, CTRL_ALT, CTRL_SHIFT};

/// Under Ctrl a character is taken as written, Shift from the flag alone:
/// the Windows console reports the unshifted key with the flag.
const UNDER_CTRL: &[Row] = &[
    (
        "Ctrl+Shift+= kitty",
        KeyCode::Char('+'),
        CTRL,
        "Ctrl++",
        "Ctrl++",
    ),
    (
        "Ctrl+Shift+= Windows",
        KeyCode::Char('='),
        CTRL_SHIFT,
        "Ctrl+Shift+=",
        "Ctrl+Shift+=",
    ),
    (
        "AltGr+q, German layout",
        KeyCode::Char('@'),
        CTRL_ALT,
        "Ctrl+Alt+@",
        "Ctrl+Alt+@",
    ),
    (
        "Ctrl+Space (NUL)",
        KeyCode::Char(' '),
        CTRL,
        "Ctrl+Space",
        "Ctrl+Space",
    ),
    (
        "Ctrl+Shift+Space",
        KeyCode::Char(' '),
        CTRL_SHIFT,
        "Ctrl+Shift+Space",
        "Ctrl+Shift+Space",
    ),
    (
        "Ctrl+\\ (decoded from 0x1C)",
        KeyCode::Char('\\'),
        CTRL,
        "Ctrl+\\",
        "Ctrl+\\",
    ),
    ("Ctrl+1", KeyCode::Char('1'), CTRL, "Ctrl+1", "Ctrl+1"),
];

#[test]
fn other_characters_under_ctrl() {
    check(UNDER_CTRL);
}

/// Under Alt alone Shift is in the character, as it is without modifiers:
/// Unix sends Alt+Shift+= as Alt+'+' with no flag (legacy bytes and kitty
/// alike), the Windows console as Alt+'+' with SHIFT, and both mean `Alt++`.
const UNDER_ALT: &[Row] = &[
    ("Alt++ Unix", KeyCode::Char('+'), ALT, "Alt++", "Alt++"),
    (
        "Alt+Shift+= Windows",
        KeyCode::Char('+'),
        ALT_SHIFT,
        "Alt++",
        "Alt++",
    ),
    (
        "Alt+Shift+1 Windows",
        KeyCode::Char('!'),
        ALT_SHIFT,
        "Alt+!",
        "Alt+!",
    ),
    (
        "Alt+Shift+Space Windows; kitty",
        KeyCode::Char(' '),
        ALT_SHIFT,
        "Alt+Space",
        "Alt+Space",
    ),
    ("Alt+:", KeyCode::Char(':'), ALT, "Alt+:", "Alt+:"),
];

#[test]
fn symbols_under_alt_carry_their_own_shift() {
    check(UNDER_ALT);
}
