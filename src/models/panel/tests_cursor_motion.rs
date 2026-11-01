//! Purpose: Panel cursor motion stops at the ends and never wraps.

use super::*;

fn listing(names: &[&str]) -> Panel {
    let entries = names
        .iter()
        .map(|name| DirEntry {
            name: (*name).to_owned(),
            ..DirEntry::default()
        })
        .collect();
    Panel::from_entries(PathBuf::from("/listing"), entries)
}

#[test]
fn next_and_previous_stop_at_the_ends() {
    let mut panel = listing(&["a", "b", "c"]);
    panel.previous();
    assert_eq!(panel.selected(), Some(0));
    panel.next();
    panel.next();
    panel.next();
    assert_eq!(panel.selected(), Some(2));
    assert_eq!(panel.get_selected_entry().expect("row").name, "c");
}

#[test]
fn an_empty_panel_ignores_motion() {
    let mut panel = listing(&[]);
    panel.next();
    panel.previous();
    panel.page_down(10);
    panel.page_up(10);
    panel.ensure_selection();
    assert_eq!(panel.selected(), None);
    assert_eq!(panel.selected_path(), None);
    assert_eq!(panel.shown_cursor(), None);
}

#[test]
fn paging_clamps_and_a_cleared_selection_starts_at_the_first_row() {
    let mut panel = listing(&["a", "b", "c", "d", "e"]);
    panel.page_down(3);
    assert_eq!(panel.selected(), Some(3));
    panel.page_down(100);
    assert_eq!(panel.selected(), Some(4));
    panel.page_up(2);
    assert_eq!(panel.selected(), Some(2));
    panel.page_up(100);
    assert_eq!(panel.selected(), Some(0));
    panel.select(None);
    panel.page_down(2);
    assert_eq!(panel.selected(), Some(0));
}

#[test]
fn a_selection_past_the_end_clears_and_parent_rows_have_no_path() {
    let mut panel = listing(&["..", "a"]);
    panel.select(Some(usize::MAX));
    assert_eq!(panel.selected(), None);
    panel.ensure_selection();
    assert_eq!(panel.selected_path(), None);
    panel.select(Some(1));
    assert_eq!(panel.selected_path(), Some(PathBuf::from("/listing/a")));
    assert_eq!(panel.shown_cursor(), Some(1));
}
