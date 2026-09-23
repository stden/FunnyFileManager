//! Unit tests: the arithmetic that decides what a click landed on.

use super::*;

#[cfg(test)]
mod tests_clicks;
#[cfg(test)]
mod tests_dialog;
#[cfg(test)]
mod tests_fkeys;

const KEYS: usize = 12;

/// `hit` on the default screen with both panels drawn.
fn both(column: u16, row: u16) -> Target {
    hit(SCREEN, column, row, KEYS, PanelsShown::Both)
}

// ─── the screen ─────────────────────────────────────────────────────────────

const SCREEN: Rect = Rect {
    x: 0,
    y: 0,
    width: 80,
    height: 24,
};

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

#[test]
fn hit_respects_a_nonzero_screen_origin() {
    let shifted = Rect::new(10, 7, 80, 24);

    assert_eq!(
        hit(shifted, 10, 7, KEYS, PanelsShown::Both),
        Target::Nothing,
        "menu bar"
    );
    assert_eq!(
        hit(shifted, 15, 10, KEYS, PanelsShown::Both),
        Target::PanelRow {
            side: PanelSide::Left,
            row: 0
        }
    );
    assert_eq!(
        hit(shifted, 10, 30, KEYS, PanelsShown::Both),
        Target::FunctionKey(1)
    );
    assert_eq!(
        hit(shifted, 9, 10, KEYS, PanelsShown::Both),
        Target::Nothing
    );
}

/// A screen with no room must answer rather than panic on the subtraction.
#[test]
fn a_degenerate_screen_hits_nothing() {
    // Each dimension on its own, not only both at once: a screen with no
    // columns has nothing to click even though it has rows, and the guard has
    // to be an `||` for that to hold.
    for (width, height) in [(0, 0), (0, 24), (80, 0)] {
        let degenerate = Rect {
            width,
            height,
            ..SCREEN
        };
        assert_eq!(
            hit(degenerate, 0, 0, KEYS, PanelsShown::Both),
            Target::Nothing,
            "{width}x{height} has nothing to click"
        );
    }

    let one_row = Rect {
        height: 1,
        ..SCREEN
    };
    // The only row is the key bar.
    assert_eq!(
        hit(one_row, 0, 0, KEYS, PanelsShown::Both),
        Target::FunctionKey(1)
    );
}

impl Target {
    fn is_row(self) -> bool {
        matches!(self, Self::PanelRow { .. })
    }

    fn side(self) -> Option<PanelSide> {
        match self {
            Self::PanelRow { side, .. } | Self::Panel(side) => Some(side),
            _ => None,
        }
    }
}

/// A row above the screen's own origin must answer rather than underflow the
/// subtraction that makes it local to `area` — the same guard the column
/// check just above it in `hit` has, but for the other axis.
#[test]
fn a_row_above_the_area_hits_nothing() {
    let shifted = Rect::new(0, 5, 80, 24);
    assert_eq!(
        hit(shifted, 10, 0, KEYS, PanelsShown::Both),
        Target::Nothing
    );
    assert_eq!(
        hit(shifted, 10, 4, KEYS, PanelsShown::Both),
        Target::Nothing,
        "still above area.y"
    );
}

/// A row at the top of `u16` must answer rather than overflow. Nothing sends
/// such an event on purpose, but a panic is not the right response to one.
#[test]
fn an_absurd_row_does_not_overflow() {
    assert_eq!(
        hit(SCREEN, 0, u16::MAX, KEYS, PanelsShown::Both),
        Target::Nothing
    );
    assert_eq!(
        hit(SCREEN, u16::MAX, u16::MAX, KEYS, PanelsShown::Both),
        Target::Nothing
    );
    // …and on a one-row screen, where `last` is 0 and `row + 1` would wrap.
    let one_row = Rect {
        height: 1,
        ..SCREEN
    };
    assert_eq!(
        hit(one_row, 0, u16::MAX, KEYS, PanelsShown::Both),
        Target::Nothing
    );
}
