//! Purpose: `from_event` for the keys that are not characters — named
//! keys, function keys, `BackTab`, the modifiers it drops, the codes it
//! refuses — the platform it reads on this host, and macOS.

use crossterm::event::{
    KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MediaKeyCode, ModifierKeyCode,
};

use super::tests_event::{check, Row, ALT, ALT_SHIFT, CTRL, CTRL_SHIFT, N, SHIFT};
use super::*;

/// `BackTab` is Tab with Shift, with or without the flag beside it.
const TAB: &[Row] = &[
    ("Tab", KeyCode::Tab, N, "Tab", "Tab"),
    (
        "Shift+Tab",
        KeyCode::BackTab,
        SHIFT,
        "Shift+Tab",
        "Shift+Tab",
    ),
    (
        "BackTab without the flag",
        KeyCode::BackTab,
        N,
        "Shift+Tab",
        "Shift+Tab",
    ),
    (
        "Tab with the flag",
        KeyCode::Tab,
        SHIFT,
        "Shift+Tab",
        "Shift+Tab",
    ),
    (
        "Ctrl+Shift+Tab",
        KeyCode::BackTab,
        CTRL_SHIFT,
        "Ctrl+Shift+Tab",
        "Ctrl+Shift+Tab",
    ),
    ("Alt+Tab", KeyCode::Tab, ALT, "Alt+Tab", "Alt+Tab"),
];

#[test]
fn tab_and_backtab() {
    check(TAB);
}

/// Named and function keys keep every bindable modifier, Shift included.
const NAMED_AND_FUNCTION: &[Row] = &[
    ("Insert", KeyCode::Insert, N, "Ins", "Ins"),
    ("Alt+Delete", KeyCode::Delete, ALT, "Alt+Del", "Alt+Del"),
    (
        "Shift+PageUp",
        KeyCode::PageUp,
        SHIFT,
        "Shift+PgUp",
        "Shift+PgUp",
    ),
    (
        "Ctrl+PageDown",
        KeyCode::PageDown,
        CTRL,
        "Ctrl+PgDn",
        "Ctrl+PgDn",
    ),
    ("Enter", KeyCode::Enter, N, "Enter", "Enter"),
    ("Esc", KeyCode::Esc, N, "Esc", "Esc"),
    (
        "Ctrl+Alt+Shift+Down",
        KeyCode::Down,
        CTRL_SHIFT.union(ALT),
        "Ctrl+Alt+Shift+Down",
        "Ctrl+Alt+Shift+Down",
    ),
    ("F1", KeyCode::F(1), N, "F1", "F1"),
    ("Ctrl+F12", KeyCode::F(12), CTRL, "Ctrl+F12", "Ctrl+F12"),
    (
        "Alt+Shift+F3",
        KeyCode::F(3),
        ALT_SHIFT,
        "Alt+Shift+F3",
        "Alt+Shift+F3",
    ),
    ("F24", KeyCode::F(24), N, "F24", "F24"),
];

#[test]
fn named_and_function_keys_keep_every_bindable_modifier() {
    check(NAMED_AND_FUNCTION);
}

/// Super, Hyper and Meta are dropped, whatever else is held.
const DROPPED: &[Row] = &[
    ("Super+F5", KeyCode::F(5), KeyModifiers::SUPER, "F5", "F5"),
    ("Meta+k", KeyCode::Char('k'), KeyModifiers::META, "k", "k"),
    (
        "Hyper+Ctrl+Down",
        KeyCode::Down,
        KeyModifiers::HYPER.union(CTRL),
        "Ctrl+Down",
        "Ctrl+Down",
    ),
    (
        "Super+Shift+Tab",
        KeyCode::BackTab,
        KeyModifiers::SUPER.union(SHIFT),
        "Shift+Tab",
        "Shift+Tab",
    ),
];

#[test]
fn super_hyper_and_meta_are_dropped() {
    check(DROPPED);
}

/// Terminal.app and iTerm2 with "Option as Meta" send Option+k as Alt+k and
/// the Delete key as Backspace; without it Option types a character, which
/// `Delivery` warns about.
const MACOS: &[Row] = &[
    ("Option+k", KeyCode::Char('k'), ALT, "Alt+K", "Alt+K"),
    (
        "Option+Shift+k",
        KeyCode::Char('K'),
        ALT_SHIFT,
        "Alt+Shift+K",
        "Alt+Shift+K",
    ),
    (
        "Delete key",
        KeyCode::Backspace,
        N,
        "Backspace",
        "Backspace",
    ),
    ("Fn+Delete", KeyCode::Delete, N, "Del", "Del"),
];

#[test]
fn macos_with_option_as_meta_reports_like_linux() {
    check(MACOS);
}

const UNBINDABLE: &[KeyCode] = &[
    KeyCode::Null,
    KeyCode::CapsLock,
    KeyCode::ScrollLock,
    KeyCode::NumLock,
    KeyCode::PrintScreen,
    KeyCode::Pause,
    KeyCode::Menu,
    KeyCode::KeypadBegin,
    KeyCode::Media(MediaKeyCode::Play),
    KeyCode::Modifier(ModifierKeyCode::LeftShift),
    KeyCode::F(0),
    KeyCode::F(25),
];

#[test]
fn unbindable_codes_give_no_chord() {
    for &code in UNBINDABLE {
        for platform in [Platform::Unix, Platform::Windows] {
            let event = KeyEvent::new(code, CTRL);
            assert_eq!(platform.chord(event), None, "{code:?} on {platform:?}");
        }
    }
}

#[test]
fn from_event_reads_the_current_platform_and_ignores_the_kind() {
    let pressed = KeyEvent::new(KeyCode::Char('K'), CTRL);
    assert_eq!(
        KeyChord::from_event(pressed),
        Platform::CURRENT.chord(pressed)
    );
    assert_eq!(Platform::CURRENT == Platform::Windows, cfg!(windows));
    let mut released = pressed;
    released.kind = KeyEventKind::Release;
    assert_eq!(
        Platform::Unix.chord(released),
        Platform::Unix.chord(pressed)
    );
}
