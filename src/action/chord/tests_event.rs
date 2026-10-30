//! Purpose: `from_event` against the raw character events of §2.2 — Unix
//! legacy bytes, the kitty protocol and the Windows console, `CapsLock`
//! included — on both platforms, whatever the host.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::*;

pub(super) const N: KeyModifiers = KeyModifiers::NONE;
pub(super) const CTRL: KeyModifiers = KeyModifiers::CONTROL;
pub(super) const ALT: KeyModifiers = KeyModifiers::ALT;
pub(super) const SHIFT: KeyModifiers = KeyModifiers::SHIFT;
pub(super) const CTRL_ALT: KeyModifiers = CTRL.union(ALT);
pub(super) const CTRL_SHIFT: KeyModifiers = CTRL.union(SHIFT);
pub(super) const ALT_SHIFT: KeyModifiers = ALT.union(SHIFT);

/// A raw event — what was pressed, the code and flags crossterm reports —
/// and the canonical text of the chord it means on Unix and on Windows.
pub(super) type Row = (
    &'static str,
    KeyCode,
    KeyModifiers,
    &'static str,
    &'static str,
);

fn text(platform: Platform, event: KeyEvent) -> String {
    platform
        .chord(event)
        .map(|chord| chord.to_string())
        .unwrap_or_default()
}

pub(super) fn check(rows: &[Row]) {
    for &(pressed, code, flags, unix, windows) in rows {
        let event = KeyEvent::new(code, flags);
        assert_eq!(text(Platform::Unix, event), unix, "{pressed} on Unix");
        assert_eq!(
            text(Platform::Windows, event),
            windows,
            "{pressed} on Windows"
        );
    }
}

/// Shift is part of the character, and a SHIFT flag beside it is dropped.
const CHARACTERS: &[Row] = &[
    ("k", KeyCode::Char('k'), N, "k", "k"),
    ("Shift+k", KeyCode::Char('K'), SHIFT, "K", "K"),
    (
        "Shift+k, CapsLock on Windows",
        KeyCode::Char('k'),
        SHIFT,
        "k",
        "k",
    ),
    ("Shift+= on Unix", KeyCode::Char('+'), N, "+", "+"),
    ("Shift+= on Windows", KeyCode::Char('+'), SHIFT, "+", "+"),
    ("space", KeyCode::Char(' '), N, "Space", "Space"),
    ("Shift+Space", KeyCode::Char(' '), SHIFT, "Space", "Space"),
    ("ж", KeyCode::Char('ж'), N, "ж", "ж"),
    ("Shift+ж", KeyCode::Char('Ж'), SHIFT, "Ж", "Ж"),
];

#[test]
fn characters_and_shift() {
    check(CHARACTERS);
}

/// A letter under Ctrl or Alt is lower-cased; Shift comes from the flag,
/// and on Unix from kitty's upper-case letter, which on Windows is `CapsLock`.
const LETTERS: &[Row] = &[
    ("Ctrl+k", KeyCode::Char('k'), CTRL, "Ctrl+K", "Ctrl+K"),
    (
        "Ctrl+Shift+k kitty; CapsLock Ctrl+k",
        KeyCode::Char('K'),
        CTRL,
        "Ctrl+Shift+K",
        "Ctrl+K",
    ),
    (
        "Ctrl+Shift+k Windows",
        KeyCode::Char('K'),
        CTRL_SHIFT,
        "Ctrl+Shift+K",
        "Ctrl+Shift+K",
    ),
    (
        "Ctrl+Shift+k flagged",
        KeyCode::Char('k'),
        CTRL_SHIFT,
        "Ctrl+Shift+K",
        "Ctrl+Shift+K",
    ),
    (
        "Alt+Shift+k legacy; Windows",
        KeyCode::Char('K'),
        ALT_SHIFT,
        "Alt+Shift+K",
        "Alt+Shift+K",
    ),
    (
        "Alt+Shift+k kitty; CapsLock Alt+k",
        KeyCode::Char('K'),
        ALT,
        "Alt+Shift+K",
        "Alt+K",
    ),
    (
        "Ctrl+Alt+k",
        KeyCode::Char('k'),
        CTRL_ALT,
        "Ctrl+Alt+K",
        "Ctrl+Alt+K",
    ),
    (
        "Ctrl+Shift+ж kitty",
        KeyCode::Char('Ж'),
        CTRL,
        "Ctrl+Shift+Ж",
        "Ctrl+Ж",
    ),
    ("Ctrl+ß", KeyCode::Char('ß'), CTRL, "Ctrl+ß", "Ctrl+ß"),
    ("Ctrl+中", KeyCode::Char('中'), CTRL, "Ctrl+中", "Ctrl+中"),
    (
        "Ctrl+Shift+中",
        KeyCode::Char('中'),
        CTRL_SHIFT,
        "Ctrl+Shift+中",
        "Ctrl+Shift+中",
    ),
];

#[test]
fn letters_under_ctrl_or_alt() {
    check(LETTERS);
}
