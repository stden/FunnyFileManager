//! Purpose: Pin the Language servers dialog's layout cell for cell, at the
//! design's 80×24 and at the sizes where its rows give way: the rules, the
//! list's half of the body, the detail pane and where it is cut, the message
//! row, and the F3 view.

#[path = "tests_layout_screens.rs"]
mod screens;

use super::*;
use screens::{
    LIST_4X3, LIST_60X12, LIST_80X15, LIST_80X16, LIST_80X24, LIST_80X3, LIST_80X4, LIST_80X5,
    LIST_80X6, LIST_80X7, LIST_80X8, VIEW_80X24, VIEW_80X4, VIEW_80X6, VIEW_80X7,
};

/// The dialog of the documentation's screenshot, drawn at `width`×`height`,
/// against `expected` rows (trailing blanks left out).
fn assert_screen(app: &mut App, (width, height): (u16, u16), expected: &[&str]) {
    let screen = buffer_string(&draw(app, width, height));
    let rows: Vec<&str> = screen.lines().map(str::trim_end).collect();
    assert_eq!(rows, expected, "{width}×{height}:\n{screen}");
}

#[test]
fn the_list_screen_at_80_by_24_is_the_designs() {
    let mut app = capture::review_app();
    assert_screen(&mut app, (80, 24), LIST_80X24);
    let buffer = draw(&mut app, 80, 24);
    assert_eq!(
        buffer[(40, 21)].bg,
        crate::ui::FAR_BLUE,
        "the panel's blue ground"
    );
    assert_eq!(fg(&buffer, "Settings: "), palette::CODE_NORMAL);
}

/// At 16 rows the detail pane keeps what fits and says where the rest is;
/// one row fewer and it gives way to a pointer to F3.
#[test]
fn the_detail_pane_is_cut_at_16_rows_and_gone_below() {
    let mut app = capture::review_app();
    assert_screen(&mut app, (80, 16), LIST_80X16);
    assert_screen(&mut app, (80, 15), LIST_80X15);
    assert_screen(&mut app, (60, 12), LIST_60X12);
}

/// The smallest window that has a frame and a key bar is 4×3; below it only
/// the ground is drawn.
#[test]
fn a_window_needs_three_rows_and_four_columns() {
    let mut app = capture::review_app();
    assert_screen(&mut app, (4, 3), LIST_4X3);
    for size in [(3, 24), (80, 2)] {
        let screen = buffer_string(&draw(&mut app, size.0, size.1));
        assert!(
            screen.chars().all(|ch| ch == ' ' || ch == '\n'),
            "{size:?}:\n{screen}"
        );
    }
}

/// F3 lists every server under the note and a rule; on a short window the
/// servers go first, then the rule, then the note, and the bottom border is
/// left alone.
#[test]
fn the_view_screen_keeps_its_rows_inside_the_frame() {
    let mut app = capture::review_app();
    press(&mut app, KeyCode::F(3));
    assert_screen(&mut app, (80, 24), VIEW_80X24);
    assert_screen(&mut app, (80, 7), VIEW_80X7);
    assert_screen(&mut app, (80, 6), VIEW_80X6);
    assert_screen(&mut app, (80, 4), VIEW_80X4);
}

/// The bottom border's corners and last `═`, and the key bar's first cells, at
/// `height` rows, 80 wide.
fn assert_bottom_intact(buffer: &Buffer, height: u16) {
    let border = height - 2;
    let bar = height - 1;
    assert_eq!(buffer[(0, border)].symbol(), "╚", "{height} rows");
    assert_eq!(buffer[(79, border)].symbol(), "╝", "{height} rows");
    for x in [0, 77, 79] {
        assert_eq!(
            buffer[(x, border)].fg,
            palette::FRAME,
            "{height} rows, column {x}"
        );
    }
    assert_eq!(buffer[(0, bar)].symbol(), "1", "{height} rows");
    assert_eq!(
        buffer[(0, bar)].bg,
        ratatui::style::Color::Black,
        "{height} rows"
    );
    assert_eq!(buffer[(1, bar)].bg, palette::CURSOR_BG, "{height} rows");
}

/// Nothing drawn inside the window spills onto its bottom border or the key
/// bar, at any height: not a rule, not a row's colours.
#[test]
fn nothing_inside_the_window_reaches_its_bottom_border_or_the_key_bar() {
    let mut app = capture::review_app();
    for height in 3..=24 {
        assert_bottom_intact(&draw(&mut app, 80, height), height);
    }
    press(&mut app, KeyCode::F(3));
    for height in 3..=24 {
        assert_bottom_intact(&draw(&mut app, 80, height), height);
    }
}

/// A window too short for every row keeps the settings row, then the message
/// row, then the rules, and never draws one row over another.
#[test]
fn a_short_window_drops_rows_without_drawing_one_over_another() {
    let mut app = capture::review_app();
    assert_screen(&mut app, (80, 3), LIST_80X3);
    assert_screen(&mut app, (80, 4), LIST_80X4);
    assert_screen(&mut app, (80, 5), LIST_80X5);
    assert_screen(&mut app, (80, 6), LIST_80X6);
    assert_screen(&mut app, (80, 7), LIST_80X7);
    assert_screen(&mut app, (80, 8), LIST_80X8);
}

/// Extensions longer than their column are cut with `…` inside it, and the
/// status keeps its place at the end of the row, at full width and narrow.
#[test]
fn extensions_too_long_for_their_column_are_cut_inside_it() {
    let machine = LspMachine::new();
    machine.write_settings(
        "lsp:\n  aaa: {command: aaa-server, extensions: [aaaa, bbbb, cccc, dddd], install: null}\n",
    );
    let mut app = machine.opened();
    for (width, expected) in [
        (
            80,
            "║ ▸ aaa                 [x] aaa-server                 .aaaa .bbbb .c… missing ║",
        ),
        (
            60,
            "║ ▸ aaa                 [x] aaa-ser… .aaaa .bbbb … missing ║",
        ),
    ] {
        let screen = buffer_string(&draw(&mut app, width, 24));
        let row = screen.lines().find(|line| line.contains("▸ aaa"));
        assert_eq!(row, Some(expected), "{screen}");
    }
}
