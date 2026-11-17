//! Purpose: Paging when the last frame drew the other screen: a list page after
//! the view moves one row, and a view scroll after the list stays at the top.

use super::*;

#[test]
fn a_list_page_after_the_view_was_drawn_moves_one_row() {
    let mut editor = editor(&json!({}));
    editor.record_drawn(Drawn::View {
        lines: 50,
        page: 20,
    });
    editor.move_selection(Step::PageDown);
    assert_eq!(editor.selected_index(), Some(1));
}

#[test]
fn a_view_scroll_after_the_list_was_drawn_stays_at_the_top() {
    let mut editor = editor(&json!({}));
    editor.record_drawn(Drawn::List { page: 4 });
    editor.set_mode(Mode::View(ScrollTop::TOP));
    editor.scroll_view(Step::PageDown);
    assert_eq!(editor.mode(), &Mode::View(ScrollTop::TOP));
}

#[test]
fn scrolling_outside_the_view_changes_nothing() {
    let mut editor = editor(&json!({}));
    editor.scroll_view(Step::Down);
    assert_eq!(editor.mode(), &Mode::Browse);
}
