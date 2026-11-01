//! The cursor bar over a marked entry, and whole-row green highlighting on
//! executable entries in table views.

use std::path::PathBuf;
use std::time::{Duration, UNIX_EPOCH};

use ratatui::{backend::TestBackend, buffer::Buffer, Terminal};

use crate::models::{FileName, FileSize, Modified};

use super::*;
use crate::models::{DirEntry, PanelView};

const FIRST_BODY_ROW: u16 = 2;

/// Every view, the four tables and the detail views drawn by `modes`.
const VIEWS: [PanelView; 10] = [
    PanelView::Brief,
    PanelView::Medium,
    PanelView::Full,
    PanelView::Wide,
    PanelView::Detailed,
    PanelView::Descriptions,
    PanelView::LongDescriptions,
    PanelView::Owners,
    PanelView::Links,
    PanelView::Alternative,
];

fn panel(view: PanelView, marked: bool) -> Panel {
    let entries = vec![DirEntry {
        name: "qq.rs".to_owned(),
        ..DirEntry::default()
    }];
    let mut panel = Panel::from_entries(PathBuf::from("/listing"), entries);
    panel.view = view;
    panel.select(Some(0));
    if marked {
        panel.marked.insert(FileName::from("qq.rs"));
    }
    panel
}

fn draw(panel: &mut Panel) -> Buffer {
    let mut terminal = Terminal::new(TestBackend::new(60, 8)).expect("test terminal");
    terminal
        .draw(|frame| render_panel(frame, panel, frame.area(), true, None))
        .expect("render panel");
    terminal.backend().buffer().clone()
}

/// The cell holding the first letter of the entry's name on the cursor row.
fn name_cell(buffer: &Buffer) -> &ratatui::buffer::Cell {
    let x = (0..60)
        .find(|&x| buffer[(x, FIRST_BODY_ROW)].symbol() == "q")
        .expect("the entry is drawn on the first body row");
    &buffer[(x, FIRST_BODY_ROW)]
}

#[test]
fn a_marked_entry_under_the_cursor_keeps_the_marked_colour_on_the_bar() {
    for view in VIEWS {
        let buffer = draw(&mut panel(view, true));
        let cell = name_cell(&buffer);
        assert_eq!(
            cell.bg,
            palette::CURSOR_BG,
            "{view:?}: still the cursor bar"
        );
        assert_eq!(cell.fg, palette::MARKED, "{view:?}: and still marked");
    }
}

#[test]
fn an_unmarked_entry_under_the_cursor_keeps_the_cursor_colours() {
    for view in VIEWS {
        let buffer = draw(&mut panel(view, false));
        let cell = name_cell(&buffer);
        assert_eq!(cell.bg, palette::CURSOR_BG, "{view:?}");
        assert_eq!(cell.fg, palette::CURSOR_FG, "{view:?}");
    }
}

/// The cursor bar as another list asks for it by mark: the bar's ground either
/// way, with the text yellow only over a marked row.
#[test]
fn the_cursor_bar_text_is_yellow_only_over_a_marked_row() {
    let marked = cursor_style(Mark::Marked);
    let unmarked = cursor_style(Mark::Unmarked);
    assert_eq!(marked.bg, Some(palette::CURSOR_BG));
    assert_eq!(unmarked.bg, Some(palette::CURSOR_BG));
    assert_eq!(marked.fg, Some(palette::MARKED));
    assert_eq!(unmarked.fg, Some(palette::CURSOR_FG));
}

#[test]
fn an_executable_row_paints_name_extension_size_date_and_time_green() {
    let entries = vec![
        DirEntry {
            name: "README.md".to_owned(),
            size: FileSize::from_bytes(9959),
            modified: Modified::new(UNIX_EPOCH + Duration::from_secs(1_700_000_000)),
            ..DirEntry::default()
        },
        DirEntry {
            name: "hosts.py".to_owned(),
            executable: true,
            size: FileSize::from_bytes(24_786),
            modified: Modified::new(UNIX_EPOCH + Duration::from_secs(1_700_000_000)),
            ..DirEntry::default()
        },
    ];
    let mut panel = Panel::from_entries(PathBuf::from("/listing"), entries);
    panel.view = PanelView::Full;
    panel.select(Some(0));
    let buffer = draw(&mut panel);
    let y = FIRST_BODY_ROW + 1;
    let row: String = (0..60).map(|x| buffer[(x, y)].symbol()).collect();
    for token in ["hosts", "py", "24786"] {
        let before = row.split_once(token).expect("token on executable row").0;
        let x = u16::try_from(before.chars().count()).expect("x fits u16");
        assert_eq!(buffer[(x, y)].fg, palette::EXECUTABLE, "{token}: {row}");
    }
    let sep_x = (1..59)
        .find(|&x| buffer[(x, y)].symbol() == "│")
        .expect("column separator");
    assert_eq!(buffer[(sep_x, y)].fg, palette::FRAME);
}
