//! Purpose: heading-bar and cursor navigation: stepping, wrapping and where the cursor may rest.
use super::*;

#[test]
fn the_bar_is_a_ring_in_both_directions() {
    let mut heading = Heading::Left;
    for _ in 0..5 {
        assert_eq!(heading.next().previous(), heading);
        heading = heading.next();
    }
    assert_eq!(heading, Heading::Left, "five steps come back round");
}

/// The cursor must never rest on a rule or on a command the program does not
/// have — pressing Enter there would do nothing, which reads as a dead key.
///
/// Unless *nothing* in the drop-down works, which is true of Options: this
/// program keeps no configuration, so all twenty-seven of its rows are greyed.
/// There the cursor rests on nothing at all, which is the honest answer.
#[test]
fn the_cursor_only_ever_rests_on_something_that_works() {
    for heading in Heading::ALL {
        let mut menu = Menu::open(heading);
        if !menu.has_anything_usable() {
            assert!(
                !menu.is_on_a_usable_row(),
                "{heading:?} offers nothing, so the cursor must claim nothing"
            );
            continue;
        }
        for _ in 0..menu.entries().len() * 2 {
            assert!(menu.is_on_a_usable_row(), "{heading:?} row {}", menu.row);
            menu.step(true);
        }
        for _ in 0..menu.entries().len() * 2 {
            assert!(menu.is_on_a_usable_row());
            menu.step(false);
        }
    }
}

#[test]
fn stepping_down_and_back_up_returns_to_the_same_row() {
    let mut menu = Menu::open(Heading::Files);
    let start = menu.row;
    menu.step(true);
    assert_ne!(menu.row, start);
    menu.step(false);
    assert_eq!(menu.row, start);
}

#[test]
fn moving_between_headings_lands_on_a_usable_row() {
    let mut menu = Menu::open(Heading::Left);
    for _ in 0..10 {
        menu.go(true);
        assert!(menu.is_on_a_usable_row() || !menu.has_anything_usable());
    }
    for _ in 0..10 {
        menu.go(false);
        assert!(menu.is_on_a_usable_row() || !menu.has_anything_usable());
    }
}
