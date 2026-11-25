//! Purpose: the editor obeys the keys it is handed, whatever they are — a
//! key given to a command runs it, and a default taken away runs nothing.

use std::collections::BTreeMap;

use crate::action::editor::{resolve, EditorCommand};
use crate::action::Bindings;
use crate::keymap::{tables, ActionName};
use crate::test_support::chord;

/// The editor's keys under a file that names `action` with `keys`.
fn with(action: &'static str, keys: &[&str]) -> Bindings<EditorCommand> {
    let chords = keys.iter().map(|key| chord(key)).collect();
    let explicit = BTreeMap::from([(ActionName::new(action), chords)]);
    tables::EDITOR.layered(&explicit).bindings
}

fn pressed(bindings: &Bindings<EditorCommand>, key: &str) -> EditorCommand {
    resolve(bindings, chord(key).to_event())
}

#[test]
fn a_key_given_to_a_command_runs_it_and_its_default_runs_nothing() {
    let keys = with("save", &["Ctrl+S"]);
    assert_eq!(pressed(&keys, "Ctrl+S"), EditorCommand::Save);
    assert_eq!(pressed(&keys, "F2"), EditorCommand::Ignore);
    assert_eq!(pressed(&keys, "F7"), EditorCommand::Search, "untouched");
}

/// The old behaviour of a chord is one line of `ffm.yml`.
#[test]
fn a_modified_key_runs_a_command_once_it_is_bound() {
    let keys = with("save", &["F2", "Shift+F2"]);
    assert_eq!(pressed(&keys, "F2"), EditorCommand::Save);
    assert_eq!(pressed(&keys, "Shift+F2"), EditorCommand::Save);
    assert_eq!(pressed(&keys, "Ctrl+F2"), EditorCommand::Ignore);
}

#[test]
fn a_key_taken_from_another_command_runs_the_one_it_was_given_to() {
    let keys = with("search", &["F2"]);
    assert_eq!(pressed(&keys, "F2"), EditorCommand::Search);
    assert_eq!(pressed(&keys, "F7"), EditorCommand::Ignore);
}

#[test]
fn an_unbound_command_has_no_key() {
    let keys = with("newline", &[]);
    assert_eq!(pressed(&keys, "Enter"), EditorCommand::Ignore);
}

/// The lookup comes before the fallback: a bound character is a command,
/// and only that character.
#[test]
fn a_bound_character_runs_its_command_instead_of_typing() {
    let keys = with("hover", &["?"]);
    assert_eq!(
        pressed(&keys, "?"),
        EditorCommand::Lsp(crate::models::LspFeature::Hover)
    );
    assert_eq!(pressed(&keys, "/"), EditorCommand::Insert('/'));
}

#[test]
fn without_a_key_at_all_the_editor_still_types() {
    let keys = Bindings::default();
    assert_eq!(pressed(&keys, "a"), EditorCommand::Insert('a'));
    assert_eq!(pressed(&keys, "Esc"), EditorCommand::Ignore);
}
