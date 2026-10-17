//! Unit tests: the list cursor moves by every list key and never leaves the
//! list, whatever it is asked.

use super::{RowCursor, RowMotion};

fn at(index: usize) -> RowCursor {
    RowCursor::at(index, usize::MAX)
}

#[test]
fn single_steps_move_one_row_and_stop_at_either_end() {
    assert_eq!(at(3).moved(RowMotion::Next, 10).index(), 4);
    assert_eq!(at(3).moved(RowMotion::Previous, 10).index(), 2);
    assert_eq!(
        at(9).moved(RowMotion::Next, 10).index(),
        9,
        "the last row stays"
    );
    assert_eq!(RowCursor::TOP.moved(RowMotion::Previous, 10).index(), 0);
}

#[test]
fn first_and_last_jump_to_the_ends() {
    assert_eq!(at(5).moved(RowMotion::First, 10).index(), 0);
    assert_eq!(at(5).moved(RowMotion::Last, 10).index(), 9);
}

#[test]
fn a_page_moves_by_its_rows_and_clamps() {
    assert_eq!(at(1).moved(RowMotion::PageDown(3), 10).index(), 4);
    assert_eq!(at(8).moved(RowMotion::PageUp(3), 10).index(), 5);
    assert_eq!(at(8).moved(RowMotion::PageDown(3), 10).index(), 9);
    assert_eq!(at(2).moved(RowMotion::PageUp(3), 10).index(), 0);
}

/// A panel too short for a page still moves, one row at a time.
#[test]
fn an_empty_page_is_one_row() {
    assert_eq!(at(4).moved(RowMotion::PageDown(0), 10).index(), 5);
    assert_eq!(at(4).moved(RowMotion::PageUp(0), 10).index(), 3);
}

#[test]
fn a_cursor_past_a_shorter_list_is_brought_back_inside_it() {
    assert_eq!(RowCursor::at(7, 3).index(), 2);
    assert_eq!(RowCursor::at(1, 3).index(), 1);
    assert_eq!(at(7).moved(RowMotion::Previous, 3).index(), 1);
    assert_eq!(at(7).moved(RowMotion::Next, 3).index(), 2);
}

#[test]
fn an_empty_list_keeps_the_cursor_on_row_zero() {
    for motion in [
        RowMotion::Next,
        RowMotion::Previous,
        RowMotion::Last,
        RowMotion::PageDown(5),
    ] {
        assert_eq!(at(4).moved(motion, 0).index(), 0, "{motion:?}");
    }
    assert_eq!(RowCursor::at(3, 0), RowCursor::TOP);
}
