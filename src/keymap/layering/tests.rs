//! Purpose: the four layers over a small table — the file's keys, a chord
//! two actions claim, defaults and the ones the file displaced — and the
//! helpers the other layering tests read them with.

use super::*;
use crate::keymap::chords::{ch, ctrl, f, named};
use crate::keymap::Origin::Ffm;
use crate::keymap::{row, Context, Group};
use crate::test_support::chord;

mod exits;
mod invariant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Screen {
    View,
    Edit,
    Copy,
    Compare,
    Close,
    Hide,
}

/// A screen with two ways out, `close` and `hide`, and one action that has
/// no key by default.
pub(super) const SCREEN: Table<Screen> = Table {
    context: Context::Viewer,
    groups: &[
        Group {
            title: "Files",
            rows: &[
                row("view", Screen::View, &[f(3)]).note(Ffm, "View"),
                row("edit", Screen::Edit, &[f(4), ch('e')]).note(Ffm, "Edit"),
                row("copy", Screen::Copy, &[f(5)]).note(Ffm, "Copy"),
                row("compare", Screen::Compare, &[]).note(Ffm, "Compare"),
            ],
        },
        Group {
            title: "Leaving",
            rows: &[
                row(
                    "close",
                    Screen::Close,
                    &[named(crate::action::chord::NamedKey::Esc), f(10)],
                )
                .note(Ffm, "Close")
                .exit(),
                row("hide", Screen::Hide, &[ctrl(ch('o'))])
                    .note(Ffm, "Hide")
                    .exit(),
            ],
        },
    ],
};

pub(super) fn name(text: &'static str) -> ActionName {
    ActionName::new(text)
}

/// What a file says: each action it names with its keys.
pub(super) fn file(entries: &[(&'static str, &[&str])]) -> Explicit {
    entries
        .iter()
        .map(|&(action, keys)| (name(action), keys.iter().map(|key| chord(key)).collect()))
        .collect()
}

/// The keys in force as text, action by action in the table's order.
pub(super) fn keys(layered: &Layered<Screen>) -> Vec<(&'static str, Vec<String>)> {
    SCREEN
        .rows()
        .map(|row| {
            let keys = layered.bindings.keys(row.command);
            (row.name.as_str(), keys.map(|key| key.to_string()).collect())
        })
        .collect()
}

pub(super) fn keys_of(layered: &Layered<Screen>, action: &str) -> Vec<String> {
    let found = keys(layered).into_iter().find(|(name, _)| *name == action);
    let Some((_, keys)) = found else {
        panic!("{action} is not an action of the screen");
    };
    keys
}

#[test]
fn without_a_file_every_action_has_its_defaults() {
    let layered = SCREEN.layered(&Explicit::new());
    assert_eq!(layered.conflicts, []);
    assert_eq!(layered.bindings, SCREEN.defaults());
    for row in SCREEN.rows() {
        let held: Vec<KeyChord> = layered.bindings.keys(row.command).collect();
        assert_eq!(held, row.defaults, "{}", row.name);
    }
}

#[test]
fn the_keys_the_file_gives_replace_the_defaults_in_the_files_order() {
    let layered = SCREEN.layered(&file(&[("edit", &["Ctrl+E", "F2"])]));
    assert_eq!(layered.conflicts, []);
    assert_eq!(keys_of(&layered, "edit"), ["Ctrl+E", "F2"]);
    assert_eq!(layered.bindings.command(chord("F4")), None);
    assert_eq!(layered.bindings.command(chord("e")), None);
    assert_eq!(keys_of(&layered, "view"), ["F3"], "an absent action");
}

#[test]
fn an_empty_list_leaves_the_action_without_a_key() {
    let layered = SCREEN.layered(&file(&[("copy", &[])]));
    assert_eq!(layered.conflicts, []);
    assert_eq!(keys_of(&layered, "copy"), Vec::<String>::new());
    assert_eq!(layered.bindings.command(chord("F5")), None);
}

#[test]
fn a_keyless_action_takes_the_key_the_file_gives_it() {
    let layered = SCREEN.layered(&file(&[("compare", &["Ctrl+D"])]));
    assert_eq!(layered.conflicts, []);
    assert_eq!(
        layered.bindings.command(chord("Ctrl+D")),
        Some(Screen::Compare)
    );
}

#[test]
fn a_key_the_file_gives_is_taken_from_the_action_that_had_it_by_default() {
    let layered = SCREEN.layered(&file(&[("view", &["F4"])]));
    assert_eq!(layered.bindings.command(chord("F4")), Some(Screen::View));
    assert_eq!(keys_of(&layered, "edit"), ["e"], "the other default stays");
    assert_eq!(layered.bindings.command(chord("F3")), None);
    assert_eq!(
        layered.conflicts,
        [Conflict::DefaultDisplaced {
            chord: chord("F4"),
            from: name("edit"),
            to: name("view"),
        }]
    );
}

#[test]
fn two_actions_swap_their_keys() {
    let layered = SCREEN.layered(&file(&[("view", &["F4"]), ("edit", &["F3"])]));
    assert_eq!(
        layered.conflicts,
        [],
        "both are named: nothing is displaced"
    );
    assert_eq!(keys_of(&layered, "view"), ["F4"]);
    assert_eq!(keys_of(&layered, "edit"), ["F3"]);
}

#[test]
fn a_key_two_named_actions_claim_goes_to_neither() {
    let layered = SCREEN.layered(&file(&[("view", &["F3", "F7"]), ("edit", &["F7", "F4"])]));
    assert_eq!(layered.bindings.command(chord("F7")), None);
    assert_eq!(keys_of(&layered, "view"), ["F3"]);
    assert_eq!(keys_of(&layered, "edit"), ["F4"]);
    assert_eq!(
        layered.conflicts,
        [Conflict::Claimed {
            chord: chord("F7"),
            first: name("view"),
            second: name("edit"),
        }]
    );
}
