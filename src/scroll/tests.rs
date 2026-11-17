//! Purpose: Pins that the two scroll offsets anchor to opposite ends.

use super::*;

#[test]
fn a_viewer_starts_at_the_top_and_walks_down() {
    let mut scroll = ScrollTop::TOP;
    assert_eq!(scroll.lines(), 0);
    scroll = scroll.down(3, 10);
    assert_eq!(scroll.lines(), 3);
    scroll = scroll.up(1);
    assert_eq!(scroll.lines(), 2);
}

#[test]
fn a_console_starts_at_the_bottom_and_walks_back() {
    let mut scroll = ScrollBack::BOTTOM;
    assert_eq!(scroll.lines(), 0);
    scroll = scroll.back(3, 10);
    assert_eq!(scroll.lines(), 3);
    scroll = scroll.forward(1);
    assert_eq!(scroll.lines(), 2);
}

#[test]
fn both_clamp_rather_than_running_off_their_end() {
    // The clamp lives on the type, so no key handler has to remember it — the
    // viewer's used to do it by hand with an `if` around the increment.
    assert_eq!(ScrollTop::TOP.down(1_000, 4).lines(), 4);
    assert_eq!(ScrollTop::TOP.up(1_000), ScrollTop::TOP);
    assert_eq!(ScrollBack::BOTTOM.back(1_000, 4).lines(), 4);
    assert_eq!(ScrollBack::BOTTOM.forward(1_000), ScrollBack::BOTTOM);
}

#[test]
fn an_empty_content_cannot_be_scrolled_anywhere() {
    assert_eq!(ScrollTop::TOP.down(5, 0), ScrollTop::TOP);
    assert_eq!(ScrollBack::BOTTOM.back(5, 0), ScrollBack::BOTTOM);
}

#[test]
fn the_two_offsets_mean_opposite_things_at_the_same_number() {
    // Three lines from the top and three lines from the bottom are different
    // places in the same content; they were the same `usize` before.
    let from_top = ScrollTop::TOP.down(3, 9);
    let from_bottom = ScrollBack::BOTTOM.back(3, 9);
    assert_eq!(from_top.lines(), from_bottom.lines());
    // Ten lines: the viewer shows line 3 onwards, the console ends at line 6.
    assert_eq!(from_top.lines(), 3, "first visible line");
    assert_eq!(
        10 - from_bottom.lines(),
        7,
        "one past the last visible line"
    );
}

#[test]
fn a_viewer_pans_right_from_the_left_edge_and_stops_there_coming_back() {
    let mut col = ScrollLeft::LEFT;
    assert_eq!(col.cells(), 0);
    col = col.right(20);
    assert_eq!(col.cells(), 20);
    col = col.left(5);
    assert_eq!(col.cells(), 15);
    // Left at the edge stays at the edge; Right has no far end to hit.
    assert_eq!(ScrollLeft::LEFT.left(1), ScrollLeft::LEFT);
    assert_eq!(col.left(1_000), ScrollLeft::LEFT);
    assert_eq!(
        ScrollLeft::LEFT.right(usize::MAX).right(1).cells(),
        usize::MAX
    );
}
