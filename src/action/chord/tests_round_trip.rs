//! Purpose: the canonical text, and the round trips that make one chord one
//! value: `Display` through `FromStr`, and `to_event` through `from_event`
//! on both platforms, over every key crossterm can report.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MediaKeyCode, ModifierKeyCode};

use super::tests::chord;
use super::*;

#[test]
fn canonical_text_orders_modifiers_and_upper_cases_letters() {
    let cases = [
        ("shift+ctrl+k", "Ctrl+Shift+K"),
        ("alt+Shift+f4", "Alt+Shift+F4"),
        ("Ctrl+\\", "Ctrl+\\"),
        ("Alt++", "Alt++"),
        ("Ctrl+Shift+=", "Ctrl+Shift+="),
        ("insert", "Ins"),
        ("Shift+pagedown", "Shift+PgDn"),
        ("Shift+k", "K"),
        ("K", "K"),
        ("k", "k"),
        ("Ctrl+ж", "Ctrl+Ж"),
        ("Ctrl+ß", "Ctrl+ß"),
        ("Alt+中", "Alt+中"),
        ("Space", "Space"),
        ("Ctrl+Space", "Ctrl+Space"),
        ("shift+tab", "Shift+Tab"),
        ("F24", "F24"),
    ];
    for (text, canonical) in cases {
        assert_eq!(chord(text).to_string(), canonical, "{text}");
    }
}

/// Every code crossterm can report that FFM could meet: F0 to F25, every
/// named code, the unbindable ones, printable ASCII, a blank, and a sample
/// of other scripts.
///
/// TODO(Rule of Three): `tests/keymap_golden_test/probes.rs` lists these
/// keys and the modifier sets again for the golden tables; see its note.
fn codes() -> Vec<KeyCode> {
    let mut codes = (0..=25).map(KeyCode::F).collect::<Vec<_>>();
    codes.extend(NamedKey::ALL.map(NamedKey::code));
    codes.extend([
        KeyCode::BackTab,
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
    ]);
    codes.extend(('!'..='~').map(KeyCode::Char));
    codes.extend("é ж Ж ß € 中".chars().map(KeyCode::Char));
    codes
}

/// Every combination of Ctrl, Alt and Shift, with and without a Super key.
fn flag_sets() -> Vec<KeyModifiers> {
    let bindable = [
        KeyModifiers::CONTROL,
        KeyModifiers::ALT,
        KeyModifiers::SHIFT,
    ];
    (0..8u8)
        .map(|bits| {
            bindable
                .iter()
                .enumerate()
                .filter(|(index, _)| bits & (1 << index) != 0)
                .fold(KeyModifiers::empty(), |flags, (_, &flag)| flags | flag)
        })
        .flat_map(|flags| [flags, flags | KeyModifiers::SUPER])
        .collect()
}

fn domain_events() -> Vec<KeyEvent> {
    codes()
        .into_iter()
        .flat_map(|code| {
            flag_sets()
                .into_iter()
                .map(move |flags| KeyEvent::new(code, flags))
        })
        .collect()
}

fn every_chord(platform: Platform) -> Vec<KeyChord> {
    domain_events()
        .into_iter()
        .filter_map(|event| platform.chord(event))
        .collect()
}

#[test]
fn display_round_trips_from_str_for_every_chord_of_the_domain() {
    for platform in [Platform::Unix, Platform::Windows] {
        let chords = every_chord(platform);
        assert!(chords.len() > 1000, "{platform:?}: {}", chords.len());
        for chord in chords {
            let text = chord.to_string();
            assert_eq!(text.parse::<KeyChord>().as_ref(), Ok(&chord), "{text}");
        }
    }
}

#[test]
fn to_event_round_trips_from_event_on_both_platforms() {
    for produced_on in [Platform::Unix, Platform::Windows] {
        for chord in every_chord(produced_on) {
            let event = chord.to_event();
            assert_eq!(Platform::Unix.chord(event), Some(chord), "{chord} on Unix");
            assert_eq!(
                Platform::Windows.chord(event),
                Some(chord),
                "{chord} on Windows"
            );
        }
    }
}

#[test]
fn both_platforms_bind_the_same_keys() {
    let bindable = |platform: Platform| {
        domain_events()
            .into_iter()
            .map(|event| platform.chord(event).is_some())
            .collect::<Vec<_>>()
    };
    assert_eq!(bindable(Platform::Unix), bindable(Platform::Windows));
}

/// A chord and the event `to_event` sends for it.
const CANONICAL_EVENTS: &[(&str, KeyCode, KeyModifiers)] = &[
    ("K", KeyCode::Char('K'), KeyModifiers::SHIFT),
    ("k", KeyCode::Char('k'), KeyModifiers::empty()),
    ("+", KeyCode::Char('+'), KeyModifiers::empty()),
    (
        "Ctrl+Shift+K",
        KeyCode::Char('k'),
        KeyModifiers::CONTROL.union(KeyModifiers::SHIFT),
    ),
    ("Alt+K", KeyCode::Char('k'), KeyModifiers::ALT),
    (
        "Alt+Shift+K",
        KeyCode::Char('K'),
        KeyModifiers::ALT.union(KeyModifiers::SHIFT),
    ),
    (
        "Alt+Shift+Ж",
        KeyCode::Char('Ж'),
        KeyModifiers::ALT.union(KeyModifiers::SHIFT),
    ),
    ("Alt++", KeyCode::Char('+'), KeyModifiers::ALT),
    ("Shift+Tab", KeyCode::BackTab, KeyModifiers::SHIFT),
    ("Tab", KeyCode::Tab, KeyModifiers::empty()),
    ("Space", KeyCode::Char(' '), KeyModifiers::empty()),
    ("Ctrl+Space", KeyCode::Char(' '), KeyModifiers::CONTROL),
    ("Alt+F3", KeyCode::F(3), KeyModifiers::ALT),
    ("Ins", KeyCode::Insert, KeyModifiers::empty()),
    (
        "Ctrl+Alt+Shift+PgDn",
        KeyCode::PageDown,
        KeyModifiers::CONTROL
            .union(KeyModifiers::ALT)
            .union(KeyModifiers::SHIFT),
    ),
];

#[test]
fn to_event_is_the_unix_terminals_event() {
    for &(text, code, flags) in CANONICAL_EVENTS {
        assert_eq!(chord(text).to_event(), KeyEvent::new(code, flags), "{text}");
    }
}
