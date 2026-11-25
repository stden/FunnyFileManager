//! Purpose: Tests for the task list's help page: it names only keys that do
//! something, and a key for every command the keymap has.

use std::collections::BTreeSet;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::tests::BINDINGS;
use super::{resolve, TaskCommand, HELP};

fn key(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
    KeyEvent::new(code, modifiers)
}

/// The keys one help token names: a key as Far writes it, `Alt+letter` for
/// every Alt+letter, `0-9` for every digit.
fn parse_help_key(token: &str) -> Vec<KeyEvent> {
    match token {
        "Alt+letter" => ('a'..='z')
            .map(|letter| key(KeyCode::Char(letter), KeyModifiers::ALT))
            .collect(),
        "0-9" => ('0'..='9')
            .map(|digit| key(KeyCode::Char(digit), KeyModifiers::NONE))
            .collect(),
        _ => vec![parse_one_key(token)],
    }
}

fn parse_one_key(token: &str) -> KeyEvent {
    let (modifiers, name) = match token.split_once('+').filter(|(_, name)| !name.is_empty()) {
        Some(("Ctrl", name)) => (KeyModifiers::CONTROL, name),
        Some(("Alt", name)) => (KeyModifiers::ALT, name),
        Some((prefix, _)) => panic!("unknown modifier {prefix:?} in {token:?}"),
        None => (KeyModifiers::NONE, token),
    };
    let code = match name {
        "Up" => KeyCode::Up,
        "Down" => KeyCode::Down,
        "PgUp" => KeyCode::PageUp,
        "PgDn" => KeyCode::PageDown,
        "Home" => KeyCode::Home,
        "End" => KeyCode::End,
        "Left" => KeyCode::Left,
        "Right" => KeyCode::Right,
        "Space" => KeyCode::Char(' '),
        "Ins" => KeyCode::Insert,
        "Del" => KeyCode::Delete,
        "Enter" => KeyCode::Enter,
        "Tab" => KeyCode::Tab,
        "Esc" => KeyCode::Esc,
        _ => function_or_character(name, token),
    };
    key(code, modifiers)
}

fn function_or_character(name: &str, token: &str) -> KeyCode {
    if let Some(number) = name.strip_prefix('F').filter(|rest| !rest.is_empty()) {
        return KeyCode::F(number.parse().expect("an F-key number"));
    }
    let mut characters = name.chars();
    match (characters.next(), characters.next()) {
        (Some(single), None) => KeyCode::Char(single),
        _ => panic!("the help names an unknown key {token:?}"),
    }
}

fn help_keys() -> Vec<(KeyEvent, &'static str)> {
    HELP.iter()
        .flat_map(|(keys, _)| keys.split(", ").map(|token| (token, *keys)))
        .flat_map(|(token, row)| parse_help_key(token).into_iter().map(move |key| (key, row)))
        .collect()
}

#[test]
fn every_key_the_help_names_does_something() {
    let keys = help_keys();
    assert!(keys.len() > HELP.len(), "every row names at least one key");
    for (key, row) in keys {
        assert_ne!(resolve(key), TaskCommand::Ignore, "{key:?} in {row:?}");
    }
}

/// A command as the help page names it: which character a search starts
/// with, or which digit a jump, is the user's.
fn topic(command: TaskCommand) -> String {
    match command {
        TaskCommand::Search(Some(_)) => "Search(Some(_))".to_owned(),
        TaskCommand::PidJump(_) => "PidJump(_)".to_owned(),
        other => format!("{other:?}"),
    }
}

#[test]
fn the_help_names_a_key_for_every_command() {
    let helped: BTreeSet<String> = help_keys()
        .into_iter()
        .map(|(key, _)| topic(resolve(key)))
        .collect();
    for &(code, modifiers, command) in BINDINGS {
        assert!(
            helped.contains(&topic(command)),
            "{command:?} ({modifiers:?} {code:?}) is not in the help"
        );
    }
}
