//! Purpose: unit tests for the tab ring, typing into the focused time field,
//! and clicks on the buttons.

use super::*;

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
        dialog.modified.current.is_empty(),
        "several files: leave the time alone"
    );
    dialog.field = AttributesField::Flag(0);
    dialog.push('2');
    assert!(dialog.modified.current.is_empty() && dialog.accessed.current.is_empty());

    dialog.field = AttributesField::Accessed;
    for ch in "2026-01-02 03:04:05".chars() {
        dialog.push(ch);
    }
    assert_eq!(dialog.accessed.current, "2026-01-02 03:04:05");
    dialog.backspace();
    assert_eq!(dialog.accessed.current, "2026-01-02 03:04:0");
    assert!(dialog.modified.current.is_empty());
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
