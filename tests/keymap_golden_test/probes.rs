//! Purpose: the domain of the golden oracle — every key crossterm can report
//! that FFM could meet, with every combination of Ctrl, Alt and Shift, as
//! the distinct chords they normalize to and the keys no chord names.

use std::collections::BTreeSet;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MediaKeyCode, ModifierKeyCode};
use funny_file_manager::action::chord::{FunctionKey, Modifier, Modifiers, NamedKey, Platform};
use funny_file_manager::action::KeyChord;

/// One row of every golden table: the key as the table names it, the event
/// the resolvers are asked about, and the events of every strict subset of
/// its modifiers, which decide whether a command is `lenient`.
pub struct Probe {
    pub text: String,
    /// The chord the row names; `None` for a key no chord can name.
    pub chord: Option<KeyChord>,
    pub event: KeyEvent,
    pub subsets: Vec<KeyEvent>,
}

/// Every row, in table order: the chords sorted by key and then by
/// modifiers, then the keys no chord names, each with every modifier set.
pub fn probes() -> Vec<Probe> {
    let mut rows: Vec<Probe> = chords().into_iter().map(chord_probe).collect();
    rows.extend(unbindable_probes());
    rows
}

/// The codes a chord can name: F1 to F24, every named key, `BackTab`,
/// printable ASCII and a sample of other scripts.
///
/// TODO(Rule of Three): `src/action/chord/tests_round_trip.rs` lists these
/// keys and the eight modifier sets too, where an integration test cannot
/// reach (`#[cfg(test)]`). A third user moves the domain behind the
/// `test-helpers` feature, for all three to read.
pub fn bindable_codes() -> Vec<KeyCode> {
    let mut codes = (1..=FunctionKey::LAST).map(KeyCode::F).collect::<Vec<_>>();
    codes.extend(NamedKey::ALL.map(NamedKey::code));
    codes.push(KeyCode::BackTab);
    codes.extend(('!'..='~').map(KeyCode::Char));
    codes.extend(['é', 'ж', 'Ж', 'ß', '€', '中'].map(KeyCode::Char));
    codes
}

/// The codes no chord names — a sample of what crossterm can report — and
/// the name the tables give each. None of them parses as a chord.
const UNBINDABLE: [(&str, KeyCode); 12] = [
    ("F0", KeyCode::F(0)),
    ("F25", KeyCode::F(25)),
    ("Null", KeyCode::Null),
    ("CapsLock", KeyCode::CapsLock),
    ("ScrollLock", KeyCode::ScrollLock),
    ("NumLock", KeyCode::NumLock),
    ("PrintScreen", KeyCode::PrintScreen),
    ("Pause", KeyCode::Pause),
    ("Menu", KeyCode::Menu),
    ("KeypadBegin", KeyCode::KeypadBegin),
    ("Media(Play)", KeyCode::Media(MediaKeyCode::Play)),
    (
        "Modifier(LeftShift)",
        KeyCode::Modifier(ModifierKeyCode::LeftShift),
    ),
];

/// Every set of Ctrl, Alt and Shift, in the order the tables list them:
/// none, Shift, Alt, Alt+Shift, Ctrl, Ctrl+Shift, Ctrl+Alt, all three.
pub fn modifier_sets() -> Vec<Modifiers> {
    let mut sets: Vec<Modifiers> = (0..8u8)
        .map(|bits| {
            Modifier::ALL
                .into_iter()
                .enumerate()
                .filter(|(index, _)| bits & (1 << index) != 0)
                .fold(Modifiers::NONE, |held, (_, modifier)| held.with(modifier))
        })
        .collect();
    sets.sort();
    sets
}

/// Ctrl, Alt and Shift as crossterm's flags.
fn flags(held: Modifiers) -> KeyModifiers {
    [
        (Modifier::Ctrl, KeyModifiers::CONTROL),
        (Modifier::Alt, KeyModifiers::ALT),
        (Modifier::Shift, KeyModifiers::SHIFT),
    ]
    .into_iter()
    .filter(|&(modifier, _)| held.contains(modifier))
    .fold(KeyModifiers::empty(), |flags, (_, flag)| flags | flag)
}

/// Every bindable code with every modifier set: the events the chords are
/// normalized from.
pub fn domain_events() -> Vec<KeyEvent> {
    bindable_codes()
        .into_iter()
        .flat_map(|code| {
            modifier_sets()
                .into_iter()
                .map(move |held| KeyEvent::new(code, flags(held)))
        })
        .collect()
}

/// Every distinct chord the domain normalizes to, on either platform.
fn chords() -> BTreeSet<KeyChord> {
    let mut chords = BTreeSet::new();
    for event in domain_events() {
        chords.extend(Platform::Unix.chord(event));
        chords.extend(Platform::Windows.chord(event));
    }
    chords
}

/// The strict subsets of `held`.
fn strict_subsets(held: Modifiers) -> impl Iterator<Item = Modifiers> {
    modifier_sets()
        .into_iter()
        .filter(move |&subset| subset != held && is_subset(subset, held))
}

fn is_subset(inner: Modifiers, outer: Modifiers) -> bool {
    Modifier::ALL
        .into_iter()
        .all(|modifier| !inner.contains(modifier) || outer.contains(modifier))
}

/// A chord's row. A subset of its modifiers is read back through the
/// terminal's event, so that `Shift` alone with a letter is the capital.
fn chord_probe(chord: KeyChord) -> Probe {
    let subsets = strict_subsets(chord.modifiers)
        .filter_map(|modifiers| {
            let subset = KeyChord {
                key: chord.key,
                modifiers,
            };
            Platform::Unix.chord(subset.to_event())
        })
        .map(KeyChord::to_event)
        .collect();
    Probe {
        text: chord.to_string(),
        chord: Some(chord),
        event: chord.to_event(),
        subsets,
    }
}

fn unbindable_probes() -> Vec<Probe> {
    UNBINDABLE
        .into_iter()
        .flat_map(|(name, code)| {
            modifier_sets()
                .into_iter()
                .map(move |held| unbindable_probe(name, code, held))
        })
        .collect()
}

/// A row for a key no chord names: the modifiers spelled as a chord would
/// spell them, then the code's name.
fn unbindable_probe(name: &str, code: KeyCode, held: Modifiers) -> Probe {
    let mut text = held.iter().fold(String::new(), |mut text, modifier| {
        text.push_str(modifier.name());
        text.push('+');
        text
    });
    text.push_str(name);
    let subsets = strict_subsets(held)
        .map(|subset| KeyEvent::new(code, flags(subset)))
        .collect();
    Probe {
        text,
        chord: None,
        event: KeyEvent::new(code, flags(held)),
        subsets,
    }
}
