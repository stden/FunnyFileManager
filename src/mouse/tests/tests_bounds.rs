//! Purpose: Out-of-range input — a shifted origin, a degenerate screen, and rows that would underflow or overflow.

use super::*;

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
