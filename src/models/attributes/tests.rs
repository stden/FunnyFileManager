//! Unit tests: the attributes dialog's rules — three-state boxes, the tab
//! ring, what a changed box asks for, and how the time fields parse.

use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use super::{AttributesDialog, AttributesField, Check, EntrySnapshot, Flag};

#[cfg(not(windows))]
const SET_FLAG: Flag = Flag::OwnerWrite;
#[cfg(windows)]
const SET_FLAG: Flag = Flag::ReadOnly;
#[cfg(not(windows))]
const CLEAR_FLAG: Flag = Flag::OtherWrite;
#[cfg(windows)]
const CLEAR_FLAG: Flag = Flag::Hidden;
#[cfg(not(windows))]
const MIXED_FLAG: Flag = Flag::GroupRead;
#[cfg(windows)]
const MIXED_FLAG: Flag = Flag::System;

fn snapshot(name: &str, bits: u32, is_dir: bool) -> EntrySnapshot {
    EntrySnapshot {
        path: PathBuf::from(name),
        bits,
        is_dir,
        owner: Some("alice".to_owned()),
        group: Some("staff".to_owned()),
        modified: Some(SystemTime::UNIX_EPOCH + Duration::from_hours(24)),
        accessed: None,
    }
}

fn flag_box(dialog: &AttributesDialog, flag: Flag) -> &super::FlagBox {
    dialog
        .flags
        .iter()
        .find(|it| it.flag == flag)
        .expect("the platform lists this flag")
}

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

/// Tab walks the boxes, the two time fields, the subfolders box and the
/// buttons, then wraps; Shift+Tab walks back.
#[test]
fn tab_walks_every_control_and_wraps() {
    let mut dialog = AttributesDialog::from_snapshots(&[snapshot("d", 0o755, true)]);
    assert_eq!(dialog.field, AttributesField::Flag(0));
    let count = dialog.flags.len();
    for _ in 0..count {
        dialog.advance(true);
    }
    assert_eq!(dialog.field, AttributesField::Modified);
    dialog.advance(true);
    assert_eq!(dialog.field, AttributesField::Accessed);
    dialog.advance(true);
    assert_eq!(dialog.field, AttributesField::Subfolders);
    dialog.advance(true);
    assert_eq!(dialog.field, AttributesField::Set);
    dialog.advance(true);
    assert_eq!(dialog.field, AttributesField::Cancel);
    dialog.advance(true);
    assert_eq!(dialog.field, AttributesField::Flag(0), "wraps");
    dialog.advance(false);
    assert_eq!(dialog.field, AttributesField::Cancel);
}

/// Without a directory the subfolders box does not apply, so the cursor
/// skips it.
#[test]
fn the_subfolders_box_is_skipped_for_plain_files() {
    let mut dialog = AttributesDialog::from_snapshots(&[snapshot("a", 0o644, false)]);
    dialog.field = AttributesField::Accessed;
    dialog.advance(true);
    assert_eq!(dialog.field, AttributesField::Set);
    dialog.advance(false);
    assert_eq!(dialog.field, AttributesField::Accessed);
    dialog.field = AttributesField::Subfolders;
    dialog.toggle();
    assert!(!dialog.process_subfolders, "a disabled box does not toggle");
}

/// Typing lands in whichever time field has the cursor and nowhere else.
#[test]
fn typing_edits_only_the_focused_time_field() {
    let mut dialog = AttributesDialog::from_snapshots(&[
        snapshot("a", 0o644, false),
        snapshot("b", 0o644, false),
    ]);
    assert!(
        dialog.modified.is_empty(),
        "several files: leave the time alone"
    );
    dialog.field = AttributesField::Flag(0);
    dialog.push('2');
    assert!(dialog.modified.is_empty() && dialog.accessed.is_empty());

    dialog.field = AttributesField::Accessed;
    for ch in "2026-01-02 03:04:05".chars() {
        dialog.push(ch);
    }
    assert_eq!(dialog.accessed, "2026-01-02 03:04:05");
    dialog.backspace();
    assert_eq!(dialog.accessed, "2026-01-02 03:04:0");
    assert!(dialog.modified.is_empty());
}

/// A single file opens with its own times filled in, in the format the
/// field parses back.
#[test]
fn a_single_file_offers_its_original_times() {
    let dialog = AttributesDialog::from_snapshots(&[snapshot("a", 0o644, false)]);
    let times = dialog.requested_times().expect("the offered text parses");
    assert_eq!(
        times.modified,
        Some(SystemTime::UNIX_EPOCH + Duration::from_hours(24))
    );
    assert_eq!(times.accessed, None, "unknown at open: left empty");
}

/// Empty fields leave the time alone; garbage is refused by name.
#[test]
fn time_fields_parse_or_are_refused() {
    let mut dialog = AttributesDialog::from_snapshots(&[snapshot("a", 0o644, false)]);
    dialog.modified.clear();
    dialog.accessed = "1999-12-31 23:59:59".to_owned();
    let times = dialog.requested_times().expect("valid");
    assert_eq!(times.modified, None);
    assert!(times.accessed.is_some());

    dialog.accessed = "yesterday".to_owned();
    let error = dialog.requested_times().expect_err("refused");
    assert!(error.to_string().contains("yesterday"), "{error}");
}

/// A click on the button row names the button before Enter.
#[test]
fn focus_button_names_set_then_cancel() {
    let mut dialog = AttributesDialog::from_snapshots(&[snapshot("a", 0o644, false)]);
    dialog.focus_button(1);
    assert_eq!(dialog.field, AttributesField::Cancel);
    dialog.focus_button(0);
    assert_eq!(dialog.field, AttributesField::Set);
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
