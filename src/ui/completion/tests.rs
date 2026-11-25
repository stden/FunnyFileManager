//! Purpose: Pin where a dropdown hangs: above the command line by preference,
//! below a form's field by preference, each flipping only for more room.

use ratatui::layout::Rect;

use super::{completion_placement, Side};

const SCREEN: Rect = Rect {
    x: 0,
    y: 0,
    width: 80,
    height: 24,
};

fn row(y: u16) -> Rect {
    Rect::new(0, y, 80, 1)
}

#[test]
fn the_command_line_list_hangs_above_unless_below_has_more_room() {
    let (area, visible) = completion_placement(SCREEN, row(22), 5, Side::Above).expect("room");
    assert_eq!((area.y, area.height, visible), (15, 7, 5));
    let (area, _) = completion_placement(SCREEN, row(0), 5, Side::Above).expect("room");
    assert_eq!(area.y, 1, "a prompt at the top drops its list below");
    let (area, _) = completion_placement(SCREEN, row(11), 5, Side::Above).expect("room");
    assert!(area.y < 11, "a tie keeps the preferred side");
}

#[test]
fn a_form_list_hangs_below_unless_above_has_more_room() {
    let (area, visible) = completion_placement(SCREEN, row(5), 3, Side::Below).expect("room");
    assert_eq!((area.y, area.height, visible), (6, 5, 3));
    let (area, visible) = completion_placement(SCREEN, row(21), 8, Side::Below).expect("room");
    assert_eq!(
        area.y + area.height,
        21,
        "flipped to end just above the field"
    );
    assert_eq!(visible, 8);
    let (area, _) = completion_placement(SCREEN, row(11), 5, Side::Below).expect("room");
    assert_eq!(area.y, 12, "a tie keeps the preferred side");
}

#[test]
fn no_room_on_either_side_draws_nothing() {
    let tiny = Rect::new(0, 0, 80, 3);
    assert_eq!(completion_placement(tiny, row(1), 4, Side::Below), None);
}

/// The room below starts under the anchor row, so a list short of room ends
/// on the screen's last row, never past it.
#[test]
fn a_list_short_of_room_below_ends_on_the_last_row() {
    let short = Rect::new(0, 0, 80, 12);
    let (area, visible) = completion_placement(short, row(3), 10, Side::Below).expect("room");
    assert_eq!((area.y, area.height, visible), (4, 8, 6));
    assert_eq!(area.bottom(), short.bottom());
}
