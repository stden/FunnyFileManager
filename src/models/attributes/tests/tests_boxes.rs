//! Purpose: unit tests for the checkboxes: what they open with across a
//! selection, how they cycle, and which bits a moved box asks for.

use super::*;

/// One file: every box is two-state and shows that file's bits.
#[test]
fn a_single_file_opens_with_its_own_bits_in_two_state_boxes() {
    let dialog = AttributesDialog::from_snapshots(&[snapshot("a", SET_FLAG.bit(), false)]);
    assert_eq!(flag_box(&dialog, SET_FLAG).current, Check::Set);
    assert_eq!(flag_box(&dialog, CLEAR_FLAG).current, Check::Clear);
    assert_eq!(dialog.owner, "alice");
    assert_eq!(dialog.group, "staff");
    assert!(!dialog.has_directories);
    assert_eq!(dialog.headline(), "a");

    let mut dialog = dialog;
    dialog.field = AttributesField::Flag(0);
    let first = dialog.flags[0].flag;
    let before = flag_box(&dialog, first).current;
    dialog.toggle();
    dialog.toggle();
    assert_eq!(
        flag_box(&dialog, first).current,
        before,
        "two-state: two toggles return to the start, never to [?]"
    );
}

/// Bits that differ across the selection open as [?] and cycle through it.
#[test]
fn differing_bits_open_mixed_and_a_mixed_box_cycles_through_three_states() {
    let mut dialog = AttributesDialog::from_snapshots(&[
        snapshot("a", SET_FLAG.bit() | MIXED_FLAG.bit(), false),
        snapshot("b", SET_FLAG.bit(), false),
    ]);
    let index = dialog
        .flags
        .iter()
        .position(|it| it.flag == MIXED_FLAG)
        .expect("platform flag");
    assert_eq!(dialog.flags[index].current, Check::Mixed);
    assert_eq!(dialog.headline(), "2 objects");

    dialog.field = AttributesField::Flag(index);
    let mut seen = vec![dialog.flags[index].current];
    for _ in 0..3 {
        dialog.toggle();
        seen.push(dialog.flags[index].current);
    }
    assert_eq!(
        seen,
        [Check::Mixed, Check::Set, Check::Clear, Check::Mixed],
        "the cycle visits every state and comes back"
    );
}

/// A directory in the selection makes every box three-state, as Far does.
#[test]
fn a_directory_makes_every_box_three_state() {
    let mut dialog = AttributesDialog::from_snapshots(&[snapshot("d", 0o755, true)]);
    assert!(dialog.has_directories);
    dialog.field = AttributesField::Flag(0);
    dialog.toggle();
    dialog.toggle();
    assert_eq!(dialog.flags[0].current, Check::Mixed);
}

/// Only boxes moved from their opening state ask for anything.
#[test]
fn only_changed_boxes_ask_for_a_bit() {
    let original = SET_FLAG.bit() | MIXED_FLAG.bit();
    let other = SET_FLAG.bit();
    let mut dialog = AttributesDialog::from_snapshots(&[snapshot("a", original, false)]);
    assert_eq!(dialog.new_bits(original), original);
    assert_eq!(
        dialog.new_bits(other),
        other,
        "untouched boxes leave other files alone"
    );

    let other_write = dialog
        .flags
        .iter()
        .position(|it| it.flag == CLEAR_FLAG)
        .expect("clear platform flag");
    dialog.field = AttributesField::Flag(other_write);
    dialog.toggle();
    assert_eq!(dialog.new_bits(original), original | CLEAR_FLAG.bit());
    assert_eq!(dialog.new_bits(other), other | CLEAR_FLAG.bit());

    let owner_write = dialog
        .flags
        .iter()
        .position(|it| it.flag == SET_FLAG)
        .expect("set platform flag");
    dialog.field = AttributesField::Flag(owner_write);
    dialog.toggle();
    assert_eq!(
        dialog.new_bits(original),
        MIXED_FLAG.bit() | CLEAR_FLAG.bit()
    );
}

/// With a folder selected, a box moved on to [?] means "leave it": no file
/// in the selection has that bit set or cleared.
#[test]
fn a_box_moved_to_leave_as_is_asks_for_nothing() {
    let bits = SET_FLAG.bit();
    let mut dialog = AttributesDialog::from_snapshots(&[snapshot("d", bits, true)]);
    let index = dialog
        .flags
        .iter()
        .position(|it| it.flag == CLEAR_FLAG)
        .expect("clear platform flag");
    dialog.field = AttributesField::Flag(index);
    dialog.toggle();

    assert_eq!(dialog.flags[index].initial, Check::Clear);
    assert_eq!(dialog.flags[index].current, Check::Mixed);
    assert_eq!(dialog.flags[index].request(), None);
    assert_eq!(dialog.new_bits(bits), bits);
    let with_flag = bits | CLEAR_FLAG.bit();
    assert_eq!(dialog.new_bits(with_flag), with_flag);
}

/// Every platform flag has a bit and a label, and the groups cover them all
/// exactly once.
#[test]
fn every_flag_is_drawn_once_with_a_bit_and_a_label() {
    let mut seen = Vec::new();
    for group in Flag::GROUPS {
        for flag in group.flags {
            assert_ne!(flag.bit(), 0);
            assert!(!flag.label().is_empty());
            assert!(!seen.contains(flag), "{flag:?} drawn twice");
            seen.push(*flag);
        }
    }
    assert!(!seen.is_empty());
}

/// Owner and group read "various" when the selection disagrees.
#[test]
fn a_mixed_selection_says_so_for_owner_and_group() {
    let mut other = snapshot("b", 0o644, false);
    other.owner = Some("bob".to_owned());
    other.group = None;
    let dialog = AttributesDialog::from_snapshots(&[snapshot("a", 0o644, false), other]);
    assert_eq!(dialog.owner, "various");
    assert_eq!(dialog.group, "various");
}
