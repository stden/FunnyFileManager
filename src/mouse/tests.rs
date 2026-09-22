//! Unit tests: the arithmetic that decides what a click landed on.

use super::*;

const KEYS: usize = 12;

/// `hit` on the default screen with both panels drawn.
fn both(column: u16, row: u16) -> Target {
    hit(SCREEN, column, row, KEYS, PanelsShown::Both)
}

// ─── click sequences ───────────────────────────────────────────────────────

#[test]
fn a_second_press_on_the_same_entry_activates_it() {
    let now = std::time::Instant::now();
    let mut clicks = PanelClicks::default();

    assert_eq!(clicks.press(PanelSide::Left, 4, now), PanelRowClick::Select);
    assert_eq!(
        clicks.press(
            PanelSide::Left,
            4,
            now + std::time::Duration::from_millis(250)
        ),
        PanelRowClick::Activate
    );
    assert_eq!(
        clicks.press(
            PanelSide::Left,
            4,
            now + std::time::Duration::from_millis(300)
        ),
        PanelRowClick::Select,
        "the third press starts a new sequence"
    );
}

#[test]
fn another_entry_or_a_late_press_starts_a_new_sequence() {
    let now = std::time::Instant::now();
    let at = |millis| now + std::time::Duration::from_millis(millis);
    let mut clicks = PanelClicks::default();

    assert_eq!(clicks.press(PanelSide::Left, 1, now), PanelRowClick::Select);
    assert_eq!(
        clicks.press(PanelSide::Right, 1, at(100)),
        PanelRowClick::Select,
        "the same row on the other panel is another entry"
    );
    assert_eq!(
        clicks.press(PanelSide::Right, 1, at(700)),
        PanelRowClick::Select,
        "a slow second press is not a double-click"
    );
    clicks.reset();
    assert_eq!(
        clicks.press(PanelSide::Right, 1, at(750)),
        PanelRowClick::Select,
        "an unrelated action breaks the sequence"
    );
}

/// The twelve cells must cover the row exactly — no gap a click can fall
/// through, and no overlap where two keys claim the same column.
#[test]
fn the_key_cells_tile_the_row_with_no_gaps_or_overlaps() {
    for width in [12, 13, 80, 85, 100, 200] {
        let widths = fkey_widths(width, KEYS);
        assert_eq!(widths.len(), KEYS);
        assert_eq!(
            widths.iter().copied().map(u32::from).sum::<u32>(),
            u32::from(width),
            "the cells must spend the whole row at width {width}"
        );

        // Every column answers with exactly one key, in order.
        let mut seen = Vec::new();
        for column in 0..width {
            seen.push(fkey_at(width, KEYS, column).expect("every column belongs to a key"));
        }
        assert!(
            seen.windows(2).all(|pair| pair[1] >= pair[0]),
            "keys must run left to right at width {width}"
        );
        assert_eq!(*seen.first().unwrap(), 1, "the row starts on F1");
        assert_eq!(*seen.last().unwrap(), 12, "and ends on F12");
    }
}

/// The remainder goes to the leftmost cells, which is what stops the last one
/// being five columns wider than its neighbours.
#[test]
fn a_remainder_widens_the_left_cells_not_the_last_one() {
    // 85 columns over 12 keys: 7 each, with 1 left over for the first.
    let widths = fkey_widths(85, KEYS);
    assert_eq!(widths[0], 8);
    assert_eq!(widths[1], 7);
    assert_eq!(*widths.last().unwrap(), 7);
}

#[test]
fn a_column_past_the_end_of_the_bar_hits_nothing() {
    assert_eq!(fkey_at(80, KEYS, 80), None);
    assert_eq!(fkey_at(80, KEYS, 999), None);
}

/// A bar narrower than the number of keys gives some cells no columns at all.
/// They must be unclickable rather than swallowing their neighbours' columns.
#[test]
fn a_bar_too_narrow_for_twelve_keys_still_answers_consistently() {
    let widths = fkey_widths(5, KEYS);
    assert_eq!(widths.iter().copied().map(u32::from).sum::<u32>(), 5);
    for column in 0..5 {
        assert!(fkey_at(5, KEYS, column).is_some(), "column {column}");
    }
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

// ─── clicks inside a dialog ─────────────────────────────────────────────────

use super::{DialogMouse, DialogTarget};

/// A dialog eight columns wide with two buttons on its last row.
fn dialog() -> DialogMouse {
    DialogMouse {
        area: Some(Rect::new(10, 5, 20, 8)),
        body: Some(Rect::new(13, 7, 14, 4)),
        buttons: vec![(0, 14, 21), (1, 22, 28)],
        buttons_y: 10,
        offset: (0, 0),
        grabbed: None,
        screen: Rect::new(0, 0, 80, 24),
    }
}

#[test]
fn a_click_on_a_button_names_it() {
    let dialog = dialog();
    assert_eq!(dialog.hit(14, 10), DialogTarget::Button(0));
    assert_eq!(dialog.hit(20, 10), DialogTarget::Button(0), "last column");
    assert_eq!(dialog.hit(22, 10), DialogTarget::Button(1));
    assert_eq!(dialog.hit(27, 10), DialogTarget::Button(1));
}

/// The gap between two buttons belongs to neither, and is somewhere a drag can
/// take hold of instead.
#[test]
fn the_gap_between_buttons_is_chrome() {
    assert_eq!(dialog().hit(21, 10), DialogTarget::Chrome);
}

#[test]
fn a_click_on_a_body_row_names_it() {
    let dialog = dialog();
    assert_eq!(dialog.hit(13, 7), DialogTarget::Row(0));
    assert_eq!(dialog.hit(20, 8), DialogTarget::Row(1));
}

/// The last two body rows are the rule and the buttons, and neither is a row
/// a caller can act on.
#[test]
fn the_rule_and_the_button_row_are_not_body_rows() {
    let dialog = dialog();
    assert_eq!(dialog.hit(13, 9), DialogTarget::Chrome, "the rule");
    assert!(matches!(
        dialog.hit(13, 10),
        DialogTarget::Chrome | DialogTarget::Button(_)
    ));
}

#[test]
fn the_frame_and_margin_are_chrome() {
    let dialog = dialog();
    assert_eq!(dialog.hit(10, 5), DialogTarget::Chrome, "top-left corner");
    assert_eq!(dialog.hit(29, 12), DialogTarget::Chrome, "bottom-right");
}

#[test]
fn anything_past_the_edge_is_outside() {
    let dialog = dialog();
    for (column, row) in [(9, 8), (30, 8), (15, 4), (15, 13)] {
        assert_eq!(
            dialog.hit(column, row),
            DialogTarget::Outside,
            "({column}, {row})"
        );
    }
}

/// With nothing drawn, every click is outside — otherwise a click on the
/// panels would land in a dialog that has already closed.
#[test]
fn a_cleared_tracker_answers_outside_everywhere() {
    let mut dialog = dialog();
    dialog.clear();
    assert_eq!(dialog.hit(15, 8), DialogTarget::Outside);
}

/// Clearing keeps where the dialog was dragged to, because the frame is being
/// redrawn; resetting forgets it, because a new dialog opens centred.
#[test]
fn clear_keeps_the_drag_and_reset_forgets_it() {
    let mut dialog = dialog();
    dialog.offset = (4, -2);
    dialog.grabbed = Some((1, 2));

    dialog.clear();
    assert_eq!(dialog.offset, (4, -2));

    dialog.reset();
    assert_eq!(dialog.offset, (0, 0));
    assert_eq!(dialog.grabbed, None);
}
