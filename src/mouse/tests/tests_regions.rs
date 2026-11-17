//! Purpose: Screen regions — the key bar, command line, menu bar, panel entry rows, and which side owns a column.

use super::*;

#[test]
fn the_bottom_row_is_the_key_bar() {
    assert_eq!(both(0, 23), Target::FunctionKey(1));
    assert_eq!(both(79, 23), Target::FunctionKey(12));
}

#[test]
fn the_row_above_it_is_the_command_line() {
    assert_eq!(both(10, 22), Target::CommandLine);
}

#[test]
fn the_top_row_belongs_to_the_menu_bar() {
    assert_eq!(both(5, 0), Target::Nothing);
}

/// Entry rows start below the frame and the column header, and stop above the
/// rule, the info row and the bottom border.
#[test]
fn panel_entry_rows_are_the_ones_between_the_chrome() {
    // Row 0 is the menu bar, row 1 the frame, row 2 the column header.
    assert_eq!(both(5, 1), Target::Panel(PanelSide::Left));
    assert_eq!(both(5, 2), Target::Panel(PanelSide::Left));

    assert_eq!(
        both(5, 3),
        Target::PanelRow {
            side: PanelSide::Left,
            row: 0
        },
        "the first entry"
    );
    assert_eq!(
        both(5, 4),
        Target::PanelRow {
            side: PanelSide::Left,
            row: 1
        }
    );

    // 24 rows: one menu row and two rows below the panels, so the panels are
    // 21 tall — screen rows 1..=21. The last three inside are the rule, the
    // info row and the bottom border.
    assert!(both(5, 18).is_row(), "still an entry");
    assert_eq!(both(5, 19), Target::Panel(PanelSide::Left));
    assert_eq!(both(5, 21), Target::Panel(PanelSide::Left));
}

/// The left panel takes the floor of the split, so an odd terminal gives the
/// extra column to the right one — the same rule `Layout` follows.
#[test]
fn the_split_puts_each_column_on_the_side_that_draws_it() {
    assert_eq!(both(39, 3).side(), Some(PanelSide::Left));
    assert_eq!(both(40, 3).side(), Some(PanelSide::Right));

    let odd = Rect {
        width: 81,
        ..SCREEN
    };
    assert_eq!(
        hit(odd, 39, 3, KEYS, PanelsShown::Both).side(),
        Some(PanelSide::Left)
    );
    assert_eq!(
        hit(odd, 40, 3, KEYS, PanelsShown::Both).side(),
        Some(PanelSide::Right)
    );
}

/// With one panel off screen (`Ctrl+F1` / `Ctrl+F2`, or a fullscreen view
/// on the active one) the other owns the whole width: a click on the right
/// half must not select — let alone activate — a panel nobody can see.
#[test]
fn a_panel_alone_on_screen_owns_every_column() {
    assert_eq!(
        hit(SCREEN, 60, 3, KEYS, PanelsShown::LeftOnly).side(),
        Some(PanelSide::Left)
    );
    assert_eq!(
        hit(SCREEN, 5, 3, KEYS, PanelsShown::RightOnly).side(),
        Some(PanelSide::Right)
    );
    assert_eq!(
        hit(SCREEN, 60, 1, KEYS, PanelsShown::LeftOnly),
        Target::Panel(PanelSide::Left),
        "the frame too"
    );
}
