//! Purpose: what holds whatever the file says — a chord runs one command,
//! a claimed chord displaces a default even when nobody gets it, and a
//! name that is no action of the table is not read.

use super::*;
use crate::keymap::tests::repeated;

#[test]
fn a_key_three_actions_claim_is_reported_against_the_first_each_time() {
    let layered = SCREEN.layered(&file(&[
        ("view", &["F7"]),
        ("edit", &["F7"]),
        ("copy", &["F7"]),
    ]));
    assert_eq!(layered.bindings.command(chord("F7")), None);
    let claimed = |second| Conflict::Claimed {
        chord: chord("F7"),
        first: name("view"),
        second: name(second),
    };
    assert_eq!(layered.conflicts, [claimed("edit"), claimed("copy")]);
}

#[test]
fn a_claimed_key_is_taken_from_its_default_holder_too() {
    let layered = SCREEN.layered(&file(&[("view", &["F5"]), ("edit", &["F5"])]));
    assert_eq!(layered.bindings.command(chord("F5")), None);
    assert_eq!(keys_of(&layered, "copy"), Vec::<String>::new());
    assert_eq!(
        layered.conflicts,
        [
            Conflict::Claimed {
                chord: chord("F5"),
                first: name("view"),
                second: name("edit"),
            },
            Conflict::DefaultDisplaced {
                chord: chord("F5"),
                from: name("copy"),
                to: name("view"),
            },
        ]
    );
}

#[test]
fn a_key_listed_twice_for_one_action_is_one_key() {
    let layered = SCREEN.layered(&file(&[("view", &["F7", "F3", "F7"])]));
    assert_eq!(layered.conflicts, []);
    assert_eq!(keys_of(&layered, "view"), ["F7", "F3"]);
}

#[test]
fn a_key_listed_twice_by_the_second_claimant_is_reported_once() {
    let layered = SCREEN.layered(&file(&[("view", &["F7"]), ("edit", &["F7", "F7"])]));
    assert_eq!(layered.conflicts.len(), 1);
    assert_eq!(layered.bindings.command(chord("F7")), None);
}

#[test]
fn a_name_that_is_no_action_of_the_table_is_not_read() {
    let layered = SCREEN.layered(&file(&[("rename", &["F3"])]));
    assert_eq!(layered.conflicts, []);
    assert_eq!(layered.bindings, SCREEN.defaults());
}

/// Files that fight over the same few keys in every way: the layers never
/// bind a chord twice, and every key in force is one the file or a default
/// named for that action.
#[test]
fn no_chord_is_bound_twice_whatever_the_file_says() {
    static KEYS: [&str; 4] = ["F3", "Esc", "Ctrl+O", "Ctrl+Shift+K"];
    const ACTIONS: [&str; 3] = ["view", "close", "hide"];
    // Each action takes one of the four keys, none, or stays absent.
    const CHOICES: usize = KEYS.len() + 2;
    for code in 0..CHOICES.pow(3) {
        let mut entries: Vec<(&'static str, &[&str])> = Vec::new();
        let mut rest = code;
        for action in ACTIONS {
            match rest % CHOICES {
                0 => {}
                1 => entries.push((action, &[])),
                key => entries.push((action, &KEYS[key - 2..key - 1])),
            }
            rest /= CHOICES;
        }
        let layered = SCREEN.layered(&file(&entries));
        let chords: Vec<KeyChord> = layered.bindings.chords().map(|(key, _)| key).collect();
        assert_eq!(repeated(chords), [], "{entries:?}");
        for way_out in ["close", "hide"] {
            assert!(!keys_of(&layered, way_out).is_empty(), "{entries:?}");
        }
    }
}
