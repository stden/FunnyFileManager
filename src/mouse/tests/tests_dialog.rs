//! Purpose: Clicks inside a dialog — buttons, body rows, chrome and outside.

use super::*;

/// A dialog eight columns wide with two buttons on its last row.
fn dialog() -> DialogMouse {
    DialogMouse {
        area: Some(Rect::new(10, 5, 20, 8)),
        body: Some(Rect::new(13, 7, 14, 4)),
        buttons: vec![(0, 14, 21), (1, 22, 28)],
        buttons_y: 10,
        offset: (0, 0),
        grabbed: None,
        screen: Rect::new(0, 0, 80, 24),
    }
}

#[test]
fn a_click_on_a_button_names_it() {
    let dialog = dialog();
    assert_eq!(dialog.hit(14, 10), DialogTarget::Button(0));
    assert_eq!(dialog.hit(20, 10), DialogTarget::Button(0), "last column");
    assert_eq!(dialog.hit(22, 10), DialogTarget::Button(1));
    assert_eq!(dialog.hit(27, 10), DialogTarget::Button(1));
}

/// The gap between two buttons belongs to neither, and is somewhere a drag can
/// take hold of instead.
#[test]
fn the_gap_between_buttons_is_chrome() {
    assert_eq!(dialog().hit(21, 10), DialogTarget::Chrome);
}

#[test]
fn a_click_on_a_body_row_names_it() {
    let dialog = dialog();
    assert_eq!(dialog.hit(13, 7), DialogTarget::Row(0));
    assert_eq!(dialog.hit(20, 8), DialogTarget::Row(1));
}

/// The last two body rows are the rule and the buttons, and neither is a row
/// a caller can act on.
#[test]
fn the_rule_and_the_button_row_are_not_body_rows() {
    let dialog = dialog();
    assert_eq!(dialog.hit(13, 9), DialogTarget::Chrome, "the rule");
    assert!(matches!(
        dialog.hit(13, 10),
        DialogTarget::Chrome | DialogTarget::Button(_)
    ));
}

#[test]
fn the_frame_and_margin_are_chrome() {
    let dialog = dialog();
    assert_eq!(dialog.hit(10, 5), DialogTarget::Chrome, "top-left corner");
    assert_eq!(dialog.hit(29, 12), DialogTarget::Chrome, "bottom-right");
}

#[test]
fn anything_past_the_edge_is_outside() {
    let dialog = dialog();
    for (column, row) in [(9, 8), (30, 8), (15, 4), (15, 13)] {
        assert_eq!(
            dialog.hit(column, row),
            DialogTarget::Outside,
            "({column}, {row})"
        );
    }
}

/// With nothing drawn, every click is outside — otherwise a click on the
/// panels would land in a dialog that has already closed.
#[test]
fn a_cleared_tracker_answers_outside_everywhere() {
    let mut dialog = dialog();
    dialog.clear();
    assert_eq!(dialog.hit(15, 8), DialogTarget::Outside);
}

/// Clearing keeps where the dialog was dragged to, because the frame is being
/// redrawn; resetting forgets it, because a new dialog opens centred.
#[test]
fn clear_keeps_the_drag_and_reset_forgets_it() {
    let mut dialog = dialog();
    dialog.offset = (4, -2);
    dialog.grabbed = Some((1, 2));

    dialog.clear();
    assert_eq!(dialog.offset, (4, -2));

    dialog.reset();
    assert_eq!(dialog.offset, (0, 0));
    assert_eq!(dialog.grabbed, None);
}
