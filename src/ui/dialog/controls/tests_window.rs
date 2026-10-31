//! Purpose: A selection in a text field is drawn between the text before and
//! after it, and is clipped to the part the field shows.

use super::*;

fn focused(caret: usize, selection: (usize, usize)) -> FieldLook {
    FieldLook {
        focused: true,
        edit: Some(FieldEdit {
            caret,
            selection: Some(selection),
        }),
    }
}

#[test]
fn a_selection_starting_at_the_caret_is_drawn_right_after_the_head() {
    let shown = window_selection("abcdef", "ab", "cdef", "abcdef", focused(2, (2, 4)));
    assert_eq!(shown, Some(("ab", "cd", "ef")));
}

#[test]
fn a_selection_that_begins_before_the_window_is_drawn_from_its_left_edge() {
    let shown = window_selection("abcdefgh", "d", "fgh", "dfgh", focused(4, (1, 6)));
    assert_eq!(shown, Some(("", "dfg", "h")));
}
