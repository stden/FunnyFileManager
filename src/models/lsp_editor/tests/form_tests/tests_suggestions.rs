//! Purpose: unit tests for the two states of a suggestion list.

use super::*;

#[test]
fn a_passive_list_becomes_active_on_down_or_up() {
    let mut list = Suggestions::passive(menu(&["a", "b", "c"]));
    assert_eq!(list.focus(), ListFocus::Field);
    list.step(Direction::Next);
    assert_eq!(list.focus(), ListFocus::List);
    assert_eq!(list.menu().selected(), 0, "down enters on the first row");

    let mut list = Suggestions::passive(menu(&["a", "b", "c"]));
    list.step(Direction::Previous);
    assert_eq!(list.menu().selected(), 2, "up enters on the last row");
    list.step(Direction::Next);
    assert_eq!(list.menu().selected(), 0, "and moving wraps");
    list.step(Direction::Previous);
    assert_eq!(list.menu().selected(), 2);
}

#[test]
fn a_refilled_list_keeps_its_focus_and_restarts_at_the_top() {
    let mut active = Suggestions::active(menu(&["a", "b"]));
    active.step(Direction::Next);
    let refilled = active.refilled(menu(&["b", "c"]));
    assert_eq!(refilled.focus(), ListFocus::List);
    assert_eq!(refilled.menu().selected(), 0);
    let passive = Suggestions::passive(menu(&["a"])).refilled(menu(&["b"]));
    assert_eq!(passive.focus(), ListFocus::Field);
}
