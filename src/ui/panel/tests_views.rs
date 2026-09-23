//! Brief, Medium, Full and Wide — Far's four table views of one listing.
//!
//! Every panel here is 42 cells wide and 8 rows tall: a 40-cell interior and a
//! three-row body, so Brief's columns are 12, 12 and 14 cells — the last one
//! takes what an uneven split leaves over, as Far's does.

use std::path::PathBuf;
use std::time::{Duration, UNIX_EPOCH};

use ratatui::{backend::TestBackend, buffer::Buffer, Terminal};

use crate::models::{FileSize, Modified, PanelView};

use super::*;

const HEADER_ROW: u16 = 1;
const FIRST_BODY_ROW: u16 = 2;
const RULE_ROW: u16 = 5;
const INFO_ROW: u16 = 6;

fn listing(view: PanelView, count: usize) -> Panel {
    let entries = (0..count)
        .map(|index| DirEntry {
            name: format!("f{index}.rs"),
            size: FileSize::from_bytes(1234),
            modified: Modified::new(UNIX_EPOCH + Duration::from_secs(1_700_000_000)),
            ..DirEntry::default()
        })
        .collect();
    let mut panel = Panel::from_entries(PathBuf::from("/listing"), entries);
    panel.view = view;
    panel.select(Some(0));
    panel
}

fn draw(panel: &mut Panel) -> Buffer {
    let mut terminal = Terminal::new(TestBackend::new(42, 8)).expect("test terminal");
    terminal
        .draw(|frame| render_panel(frame, panel, frame.area(), true, None))
        .expect("render panel");
    terminal.backend().buffer().clone()
}

/// Row `y` without its two frame cells.
fn interior(buffer: &Buffer, y: u16) -> String {
    (1..41).map(|x| buffer[(x, y)].symbol()).collect()
}

/// The frame cells too, for the rule whose ends join the border.
fn whole(buffer: &Buffer, y: u16) -> String {
    (0..42).map(|x| buffer[(x, y)].symbol()).collect()
}

#[test]
fn brief_flows_the_entries_down_three_name_columns() {
    let mut panel = listing(PanelView::Brief, 7);
    let screen = draw(&mut panel);
    assert_eq!(
        interior(&screen, FIRST_BODY_ROW),
        "f0        rs│f3        rs│f6          rs"
    );
    assert_eq!(
        interior(&screen, FIRST_BODY_ROW + 2),
        "f2        rs│f5        rs│              ",
        "a column with no entry left still carries its rule"
    );
}

#[test]
fn brief_titles_every_name_column_and_closes_each_rule() {
    let mut panel = listing(PanelView::Brief, 7);
    let screen = draw(&mut panel);
    assert_eq!(
        interior(&screen, HEADER_ROW),
        "n   Name    │    Name    │     Name     "
    );
    assert_eq!(
        whole(&screen, RULE_ROW),
        "╟────────────┴────────────┴──────────────╢"
    );
}

#[test]
fn medium_splits_the_panel_into_two_name_columns() {
    let mut panel = listing(PanelView::Medium, 4);
    let screen = draw(&mut panel);
    assert_eq!(
        interior(&screen, HEADER_ROW),
        "n       Name       │        Name        "
    );
    assert_eq!(
        interior(&screen, FIRST_BODY_ROW),
        "f0               rs│f3                rs"
    );
}

#[test]
fn wide_shows_the_name_and_the_size_only() {
    let mut panel = listing(PanelView::Wide, 2);
    let screen = draw(&mut panel);
    assert_eq!(
        interior(&screen, HEADER_ROW),
        "n              Name              │ Size "
    );
    assert_eq!(
        interior(&screen, FIRST_BODY_ROW),
        "f0                             rs│  1234"
    );
}

#[test]
fn the_brief_cursor_covers_only_its_own_column() {
    let mut panel = listing(PanelView::Brief, 7);
    panel.select(Some(3));
    let screen = draw(&mut panel);
    assert_eq!(screen[(14, FIRST_BODY_ROW)].bg, palette::CURSOR_BG);
    assert_eq!(screen[(25, FIRST_BODY_ROW)].bg, palette::CURSOR_BG);
    assert_ne!(screen[(13, FIRST_BODY_ROW)].bg, palette::CURSOR_BG, "rule");
    assert_ne!(screen[(1, FIRST_BODY_ROW)].bg, palette::CURSOR_BG);
    assert_ne!(screen[(14, FIRST_BODY_ROW + 1)].bg, palette::CURSOR_BG);
}

/// Far scrolls a multi-column view one entry at a time: the listing reflows
/// through the columns rather than jumping a whole column.
#[test]
fn moving_past_the_last_cell_scrolls_brief_by_one_entry() {
    let mut panel = listing(PanelView::Brief, 12);
    panel.select(Some(9));
    let screen = draw(&mut panel);
    assert!(interior(&screen, FIRST_BODY_ROW).starts_with("f1 "));
}

fn info_details(entry: &DirEntry) -> String {
    format!("  1234 {} {}", entry.modified.date(), entry.modified.time())
}

/// Far's status line shows what the columns do not: Brief and Medium have
/// no room for size, date or time, so the current file's are shown here.
#[test]
fn the_brief_and_medium_info_row_adds_size_date_and_time() {
    for view in [PanelView::Brief, PanelView::Medium] {
        let mut panel = listing(view, 3);
        let expected = info_details(&panel.entries[0]);
        let info = interior(&draw(&mut panel), INFO_ROW);
        assert!(info.starts_with("f0.rs "), "{view:?}: {info:?}");
        assert!(info.ends_with(&expected), "{view:?}: {info:?}");
    }
}

#[test]
fn the_wide_info_row_adds_date_and_time_and_full_adds_nothing() {
    let mut wide = listing(PanelView::Wide, 3);
    let entry = wide.entries[0].clone();
    let info = interior(&draw(&mut wide), INFO_ROW);
    assert!(info.ends_with(&format!(
        " {} {}",
        entry.modified.date(),
        entry.modified.time()
    )));
    assert!(!info.contains("1234"), "{info:?}");

    let mut full = listing(PanelView::Full, 3);
    assert_eq!(interior(&draw(&mut full), INFO_ROW).trim_end(), "f0.rs");
}

#[test]
fn a_message_replaces_the_whole_info_row() {
    let mut panel = listing(PanelView::Brief, 3);
    let mut terminal = Terminal::new(TestBackend::new(42, 8)).expect("test terminal");
    terminal
        .draw(|frame| {
            let message = PathMessage::from("Copied".to_owned());
            render_panel(frame, &mut panel, frame.area(), true, Some(&message));
        })
        .expect("render panel");
    let info = interior(terminal.backend().buffer(), INFO_ROW);
    assert_eq!(info.trim_end(), "Copied");
}

/// The mouse reads the same geometry the renderer draws: the rule belongs to
/// the column on its left, and the wider last column is all clickable.
#[test]
fn stripe_at_follows_the_drawn_columns() {
    let interior = Cells::new(40);
    let stripes: Vec<usize> = [0, 11, 12, 13, 25, 26, 39]
        .into_iter()
        .map(|x| stripe_at(PanelView::Brief, interior, x))
        .collect();
    assert_eq!(stripes, [0, 0, 0, 1, 1, 2, 2]);
    assert_eq!(stripe_at(PanelView::Full, interior, 39), 0);
}
