//! Purpose: A menu opened from the right panel anchors past the middle of the
//! screen.

use super::*;

#[test]
fn a_menu_on_the_right_side_starts_past_the_middle_of_the_screen() {
    let screen = Rect::new(0, 0, 120, 30);
    let placed = place(screen, PanelSide::Right, (0, 0)).expect("room for the menu");
    assert!(placed.x >= screen.width / 2, "{placed:?}");
}
