//! Purpose: `Bindings` — the lookup is exact, a chord runs one command, a
//! command's keys keep their order, and a key event is read through its
//! chord.

use crossterm::event::{KeyCode, KeyModifiers};

use super::*;
use crate::test_support::chord;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Command {
    Copy,
    Rename,
    Quit,
}

fn bound(pairs: &[(Command, &str)]) -> Bindings<Command> {
    let mut bindings = Bindings::default();
    for &(command, text) in pairs {
        bindings.bind(command, chord(text));
    }
    bindings
}

fn keys(bindings: &Bindings<Command>, command: Command) -> Vec<String> {
    bindings
        .keys(command)
        .map(|chord| chord.to_string())
        .collect()
}

#[test]
fn nothing_is_bound_until_a_chord_is_given() {
    let bindings = Bindings::<Command>::default();
    assert_eq!(bindings.command(chord("F5")), None);
    assert_eq!(bindings.chords().count(), 0);
    assert_eq!(keys(&bindings, Command::Copy), Vec::<String>::new());
}

#[test]
fn a_chord_runs_its_command_and_only_with_exactly_its_modifiers() {
    let bindings = bound(&[(Command::Copy, "F5"), (Command::Rename, "Shift+F6")]);
    assert_eq!(bindings.command(chord("F5")), Some(Command::Copy));
    assert_eq!(bindings.command(chord("Shift+F6")), Some(Command::Rename));
    for other in ["Shift+F5", "Ctrl+F5", "Alt+F5", "Ctrl+Alt+Shift+F5", "F6"] {
        assert_eq!(bindings.command(chord(other)), None, "{other}");
    }
}

#[test]
fn a_command_keeps_its_keys_in_the_order_they_were_given() {
    let bindings = bound(&[
        (Command::Quit, "F10"),
        (Command::Copy, "F5"),
        (Command::Quit, "Esc"),
        (Command::Quit, "q"),
    ]);
    assert_eq!(keys(&bindings, Command::Quit), ["F10", "Esc", "q"]);
    assert_eq!(keys(&bindings, Command::Copy), ["F5"]);
    assert_eq!(keys(&bindings, Command::Rename), Vec::<String>::new());
    let all: Vec<(String, Command)> = bindings
        .chords()
        .map(|(chord, command)| (chord.to_string(), command))
        .collect();
    assert_eq!(
        all,
        [
            ("F10".to_owned(), Command::Quit),
            ("F5".to_owned(), Command::Copy),
            ("Esc".to_owned(), Command::Quit),
            ("q".to_owned(), Command::Quit),
        ]
    );
}

#[test]
fn binding_a_held_chord_takes_it_from_its_holder() {
    let mut bindings = bound(&[(Command::Copy, "F5"), (Command::Copy, "F6")]);
    assert_eq!(bindings.bind(Command::Rename, chord("F7")), None);
    assert_eq!(
        bindings.bind(Command::Rename, chord("F5")),
        Some(Command::Copy)
    );
    assert_eq!(bindings.command(chord("F5")), Some(Command::Rename));
    assert_eq!(keys(&bindings, Command::Copy), ["F6"]);
    assert_eq!(keys(&bindings, Command::Rename), ["F7", "F5"]);
    assert_eq!(bindings.chords().count(), 3, "no chord is held twice");
}

#[test]
fn binding_a_chord_to_its_holder_again_changes_nothing() {
    let mut bindings = bound(&[(Command::Quit, "F10"), (Command::Quit, "Esc")]);
    let before = bindings.clone();
    assert_eq!(
        bindings.bind(Command::Quit, chord("F10")),
        Some(Command::Quit)
    );
    assert_eq!(bindings, before);
    assert_eq!(keys(&bindings, Command::Quit), ["F10", "Esc"]);
}

#[test]
fn a_key_event_is_read_through_its_chord() {
    let bindings = bound(&[
        (Command::Copy, "Shift+Tab"),
        (Command::Rename, "Ctrl+R"),
        (Command::Quit, "K"),
    ]);
    let shift_tab = KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT);
    assert_eq!(bindings.resolve(shift_tab), Some(Command::Copy));
    let ctrl_r = KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL);
    assert_eq!(bindings.resolve(ctrl_r), Some(Command::Rename));
    let capital = KeyEvent::new(KeyCode::Char('K'), KeyModifiers::SHIFT);
    assert_eq!(bindings.resolve(capital), Some(Command::Quit));
    let small = KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE);
    assert_eq!(bindings.resolve(small), None);
}

#[test]
fn a_key_no_chord_names_runs_nothing() {
    let bindings = bound(&[(Command::Copy, "F5")]);
    for code in [KeyCode::CapsLock, KeyCode::Null, KeyCode::F(25)] {
        let event = KeyEvent::new(code, KeyModifiers::NONE);
        assert_eq!(bindings.resolve(event), None, "{code:?}");
    }
    let super_f5 = KeyEvent::new(KeyCode::F(5), KeyModifiers::SUPER);
    assert_eq!(bindings.resolve(super_f5), Some(Command::Copy));
}
