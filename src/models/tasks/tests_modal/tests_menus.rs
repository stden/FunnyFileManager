//! Purpose: unit tests for menu cursors that stay on a row and the row each menu
//! opens on.

use super::*;

#[test]
fn a_menu_cursor_is_always_on_a_row() {
    assert_eq!(MenuCursor::at(0, 0), None);
    assert_eq!(MenuCursor::at(7, 3).map(MenuCursor::index), Some(2));
    assert_eq!(MenuCursor::at(1, 3).map(MenuCursor::index), Some(1));
}

#[test]
fn the_sort_menu_opens_on_the_key_in_force_and_clamps_moves() {
    let mut menu = TaskMenu::sort(Sort::new(SortKey::CpuTime, SortOrder::Reversed));
    assert_eq!(menu.cursor().index(), 8);
    assert_eq!(menu.row_count(), 13);
    assert_eq!(TaskMenu::SORT_ROWS, 13);
    menu.select(99);
    assert_eq!(menu.cursor().index(), 12, "Reverse order is the last row");
}

#[test]
fn the_user_menu_opens_on_the_scope_in_force() {
    let owners = vec![Owner::Named("alice".into()), Owner::Named("bob".into())];
    let all = TaskMenu::user(owners.clone(), &UserScope::All);
    assert_eq!((all.cursor().index(), all.row_count()), (0, 3));
    let bob = TaskMenu::user(owners.clone(), &UserScope::Only(Owner::Named("bob".into())));
    assert_eq!(bob.cursor().index(), 2);
    let gone = TaskMenu::user(owners, &UserScope::Only(Owner::Uid(4_000_000_000)));
    assert_eq!(
        gone.cursor().index(),
        0,
        "an owner no longer listed falls back to All"
    );
}

#[cfg(unix)]
#[test]
fn the_signal_menu_opens_on_sigterm() {
    let menu = TaskMenu::signal(vim()).expect("Unix offers signals");
    assert_eq!(menu.cursor().index(), 0);
    assert_eq!(menu.row_count(), Signal::OFFERED.len());
    assert_eq!(Signal::OFFERED.first(), Some(&Signal::Terminate));
}
