//! Purpose: a way out of a screen never ends without a key a terminal
//! delivers — its defaults come back, from whoever holds them.

use super::*;

fn restored(action: &'static str) -> Conflict {
    Conflict::ExitRestored {
        action: name(action),
    }
}

#[test]
fn a_way_out_left_without_a_key_gets_its_defaults_back() {
    let layered = SCREEN.layered(&file(&[("close", &[])]));
    assert_eq!(keys_of(&layered, "close"), ["Esc", "F10"]);
    assert_eq!(layered.conflicts, [restored("close")]);
}

#[test]
fn a_way_out_may_move_to_a_key_every_terminal_delivers() {
    let layered = SCREEN.layered(&file(&[("close", &["q"]), ("hide", &["F12"])]));
    assert_eq!(layered.conflicts, []);
    assert_eq!(keys_of(&layered, "close"), ["q"]);
    assert_eq!(keys_of(&layered, "hide"), ["F12"]);
    assert_eq!(layered.bindings.command(chord("Esc")), None);
}

/// Ctrl+Shift+Q arrives as Ctrl+Q without the kitty protocol, Ctrl+I as
/// Tab, a modified F3 as nothing and F13 not at all.
#[test]
fn a_key_some_terminals_do_not_deliver_does_not_count() {
    for undelivered in ["Ctrl+Shift+Q", "Ctrl+I", "Shift+F3", "F13", "Ctrl++"] {
        let layered = SCREEN.layered(&file(&[("close", &[undelivered])]));
        assert_eq!(
            keys_of(&layered, "close"),
            [undelivered, "Esc", "F10"],
            "the file's key stays, for the terminals that send it"
        );
        assert_eq!(layered.conflicts, [restored("close")], "{undelivered}");
    }
}

#[test]
fn restored_defaults_are_taken_back_from_the_action_the_file_gave_them_to() {
    let layered = SCREEN.layered(&file(&[("close", &[]), ("view", &["Esc", "F3"])]));
    assert_eq!(keys_of(&layered, "close"), ["Esc", "F10"]);
    assert_eq!(keys_of(&layered, "view"), ["F3"]);
    assert_eq!(layered.conflicts, [restored("close")]);
}

#[test]
fn a_way_out_whose_only_key_two_actions_claim_is_restored() {
    let layered = SCREEN.layered(&file(&[("close", &["F2"]), ("view", &["F2"])]));
    assert_eq!(keys_of(&layered, "close"), ["Esc", "F10"]);
    assert_eq!(keys_of(&layered, "view"), Vec::<String>::new());
    assert_eq!(
        layered.conflicts,
        [
            Conflict::Claimed {
                chord: chord("F2"),
                first: name("view"),
                second: name("close"),
            },
            restored("close"),
        ]
    );
}

/// `close` takes `hide`'s only default and `hide` is left bare: restoring
/// `hide` takes Ctrl+O back, which leaves `close` bare in its turn.
#[test]
fn one_way_out_restored_at_the_cost_of_the_other_restores_both() {
    let layered = SCREEN.layered(&file(&[("close", &["Ctrl+O"]), ("hide", &[])]));
    assert_eq!(keys_of(&layered, "hide"), ["Ctrl+O"]);
    assert_eq!(keys_of(&layered, "close"), ["Esc", "F10"]);
    assert_eq!(layered.conflicts, [restored("hide"), restored("close")]);
}

#[test]
fn the_ways_out_may_swap_their_keys() {
    let layered = SCREEN.layered(&file(&[("close", &["Ctrl+O"]), ("hide", &["Esc"])]));
    assert_eq!(layered.conflicts, []);
    assert_eq!(keys_of(&layered, "close"), ["Ctrl+O"]);
    assert_eq!(keys_of(&layered, "hide"), ["Esc"]);
}

#[test]
fn an_action_that_is_no_way_out_may_be_left_without_a_key() {
    let layered = SCREEN.layered(&file(&[("view", &[]), ("edit", &["Ctrl+Shift+E"])]));
    assert_eq!(layered.conflicts, []);
    assert_eq!(keys_of(&layered, "view"), Vec::<String>::new());
    assert_eq!(keys_of(&layered, "edit"), ["Ctrl+Shift+E"]);
}
