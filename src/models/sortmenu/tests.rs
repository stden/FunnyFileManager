//! Unit tests: the sort menu's rows, its cursor and what it ticks.

use super::{SortMenu, SortOrder, SortRow};
use crate::models::{PanelSide, SortMode};

#[test]
fn the_rows_are_fars_nine_modes_and_then_the_order_switch() {
    let modes: Vec<SortMode> = SortRow::ALL
        .iter()
        .filter_map(|row| match row {
            SortRow::Mode(mode) => Some(*mode),
            SortRow::Reverse => None,
        })
        .collect();
    assert_eq!(modes, SortMode::ALL.to_vec());
    assert_eq!(SortRow::ALL.last(), Some(&SortRow::Reverse));
    assert_eq!(SortRow::ALL.len(), SortMode::ALL.len() + 1);
}

/// Far's help lists Ctrl+F3 … Ctrl+F11 in menu order; the reverse row has
/// no key of its own, only the `*` that flips the order.
#[test]
fn each_mode_row_names_its_ctrl_f_key() {
    assert_eq!(SortRow::Mode(SortMode::Name).shortcut(), "Ctrl+F3");
    assert_eq!(
        SortRow::Mode(SortMode::ModificationTime).shortcut(),
        "Ctrl+F5"
    );
    assert_eq!(SortRow::Mode(SortMode::Size).shortcut(), "Ctrl+F6");
    assert_eq!(SortRow::Mode(SortMode::Owner).shortcut(), "Ctrl+F11");
    assert_eq!(SortRow::Reverse.shortcut(), "*");
}

/// The hotkey is the panel header's letter, so the same letter means the
/// same mode everywhere.
#[test]
fn a_mode_rows_hotkey_is_its_header_letter() {
    for mode in SortMode::ALL {
        assert_eq!(SortRow::Mode(mode).hotkey(), mode.letter());
        assert!(SortRow::Mode(mode)
            .label()
            .to_lowercase()
            .starts_with(mode.letter()));
    }
    assert_eq!(SortRow::Reverse.hotkey(), 'r');
    assert_eq!(SortRow::Reverse.label(), "Reverse order");
}

#[test]
fn the_menu_opens_on_the_panels_current_mode() {
    let menu = SortMenu::open(SortMode::Size, SortOrder::Reversed, PanelSide::Right);
    assert_eq!(menu.row(), SortRow::Mode(SortMode::Size));
    assert_eq!(menu.mode, SortMode::Size);
    assert_eq!(menu.order, SortOrder::Reversed);
    assert_eq!(menu.side, PanelSide::Right, "it remembers which panel");
}

#[test]
fn the_ticks_follow_the_snapshot_taken_when_it_opened() {
    let menu = SortMenu::open(SortMode::Extension, SortOrder::Reversed, PanelSide::Left);
    assert!(menu.is_ticked(SortRow::Mode(SortMode::Extension)));
    assert!(!menu.is_ticked(SortRow::Mode(SortMode::Name)));
    assert!(menu.is_ticked(SortRow::Reverse));

    let direct = SortMenu::open(SortMode::Name, SortOrder::Direct, PanelSide::Left);
    assert!(!direct.is_ticked(SortRow::Reverse));
}

#[test]
fn the_cursor_wraps_at_both_ends() {
    let mut menu = SortMenu::open(SortMode::Name, SortOrder::Direct, PanelSide::Left);
    menu.up();
    assert_eq!(menu.row(), SortRow::Reverse, "up from the top wraps");
    menu.down();
    assert_eq!(menu.row(), SortRow::Mode(SortMode::Name));
    menu.last();
    assert_eq!(menu.row(), SortRow::Reverse);
    menu.down();
    assert_eq!(
        menu.row(),
        SortRow::Mode(SortMode::Name),
        "down from the bottom wraps"
    );
    menu.first();
    assert_eq!(menu.cursor, 0);
}

/// A cursor past the end — a default menu built by hand — still names a row.
#[test]
fn a_stray_cursor_is_read_as_the_last_row() {
    let menu = SortMenu {
        cursor: 99,
        ..SortMenu::default()
    };
    assert_eq!(menu.row(), SortRow::Reverse);
}

#[test]
fn a_hotkey_finds_its_row_in_either_case() {
    assert_eq!(SortRow::by_hotkey('s'), Some(SortRow::Mode(SortMode::Size)));
    assert_eq!(SortRow::by_hotkey('S'), Some(SortRow::Mode(SortMode::Size)));
    assert_eq!(SortRow::by_hotkey('r'), Some(SortRow::Reverse));
    assert_eq!(SortRow::by_hotkey('z'), None);
}

/// Far's own mnemonic for the row — "e&Xtension" — still works, next to the
/// header letter the label shows.
#[test]
fn fars_x_names_the_extension_row_as_well_as_e() {
    let extension = Some(SortRow::Mode(SortMode::Extension));
    assert_eq!(SortRow::by_hotkey('e'), extension);
    assert_eq!(SortRow::by_hotkey('x'), extension);
    assert_eq!(SortRow::by_hotkey('X'), extension);
}

/// `Ctrl+F<n>` inside the menu names the row whose label shows that key —
/// the inverse of `shortcut`, so the two cannot disagree.
#[test]
fn a_function_key_names_the_row_that_shows_it() {
    for row in SortRow::ALL {
        let SortRow::Mode(_) = row else {
            continue;
        };
        let number: u8 = row
            .shortcut()
            .trim_start_matches("Ctrl+F")
            .parse()
            .expect("a Ctrl+F key");
        assert_eq!(SortRow::by_function_key(number), Some(row), "{row:?}");
    }
    assert_eq!(SortRow::by_function_key(2), None, "before the first mode");
    assert_eq!(
        SortRow::by_function_key(12),
        None,
        "Ctrl+F12 is the menu itself"
    );
    assert_eq!(SortRow::by_function_key(0), None);
}

#[test]
fn an_order_is_a_bool_only_at_the_panel_boundary() {
    assert_eq!(SortOrder::from_reversed(true), SortOrder::Reversed);
    assert_eq!(SortOrder::from_reversed(false), SortOrder::Direct);
    assert!(SortOrder::Reversed.is_reversed());
    assert_eq!(SortOrder::Direct.toggled(), SortOrder::Reversed);
    assert_eq!(SortOrder::Reversed.toggled(), SortOrder::Direct);
}
