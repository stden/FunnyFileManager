//! Unit tests: Far's full-screen viewer status line on row 0 — the path, the
//! fields, the percent, narrow widths — rendered through the real `ui` path,
//! plus the helpers `tests_body` shares.

use std::path::PathBuf;

use ratatui::backend::TestBackend;
use ratatui::style::Color;
use ratatui::Terminal;

use super::super::ui;
use super::*;
use crate::models::{App, Overlay};
use crate::scroll::Scrolled;

/// A file viewer over `bytes`, not touching the disk: the highlighter picks its
/// language from the name alone.
pub(super) fn file_viewer(path: &str, bytes: &[u8]) -> Viewer {
    Viewer::file(PathBuf::from(path), bytes)
}

pub(super) fn app_with(viewer: Viewer) -> App {
    let mut app = App::new().expect("test app");
    app.set_overlay(Overlay::Viewer(viewer));
    app
}

pub(super) fn draw(app: &mut App, width: u16, height: u16) -> Terminal<TestBackend> {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("test terminal");
    terminal.draw(|f| ui(f, app)).expect("draw");
    terminal
}

/// One buffer row as a string, trailing blanks trimmed.
pub(super) fn row_text(terminal: &Terminal<TestBackend>, y: u16) -> String {
    let buf = terminal.backend().buffer();
    (0..buf.area.width)
        .map(|x| buf[(x, y)].symbol())
        .collect::<String>()
        .trim_end()
        .to_owned()
}

/// The foreground of every drawn (non-blank) cell on `y`.
pub(super) fn row_foregrounds(terminal: &Terminal<TestBackend>, y: u16) -> Vec<Color> {
    let buf = terminal.backend().buffer();
    (0..buf.area.width)
        .filter(|&x| buf[(x, y)].symbol() != " ")
        .map(|x| buf[(x, y)].fg)
        .collect()
}

const CLOCK: &str = " 1:34 PM";

// ── The status line, as a string ─────────────────────────────────────────

#[test]
fn the_bar_is_the_path_then_the_far_fields_flush_right() {
    let viewer = file_viewer("/tmp/x/main.rs", b"fn main() {}");
    let row = bar_row(&viewer, Cells::new(80), CLOCK, 23);
    assert_eq!(
        row,
        format!(
            "/tmp/x/main.rs{}│t│UTF-8│12      │Col 0   │100% │ 1:34 PM",
            " ".repeat(25)
        )
    );
    assert_eq!(Cells::of(&row).get(), 80);
}

#[test]
fn a_single_byte_file_names_its_code_page_in_the_same_column() {
    let viewer = file_viewer("/tmp/x/a.txt", b"caf\xe9");
    let row = bar_row(&viewer, Cells::new(60), CLOCK, 23);
    assert!(
        row.ends_with("│t│Windows-1251│4       │Col 0   │100% │ 1:34 PM"),
        "{row:?}"
    );
}

#[test]
fn the_percent_is_where_the_top_row_is_until_the_last_line_shows() {
    let text = (0..100)
        .map(|i| i.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    let mut viewer = Viewer::file(PathBuf::from("/n"), text.as_bytes());
    let page = 23;
    assert!(bar_row(&viewer, Cells::new(80), CLOCK, page).contains("│0%   │"));
    viewer.scroll_down(50, page);
    assert!(bar_row(&viewer, Cells::new(80), CLOCK, page).contains("│50%  │"));
    viewer.scroll_to_bottom(page);
    assert_eq!(viewer.scroll.lines(), 77);
    assert!(bar_row(&viewer, Cells::new(80), CLOCK, page).contains("│100% │"));
}

/// The path gives way from its head, as Far cuts it: the file name at its
/// tail is what says which file this is.
#[test]
fn a_narrow_bar_cuts_the_path_from_the_left_and_keeps_the_fields_whole() {
    let viewer = file_viewer("/tmp/x/main.rs", b"fn main() {}");
    let row = bar_row(&viewer, Cells::new(50), CLOCK, 23);
    assert_eq!(row, "…/main.rs│t│UTF-8│12      │Col 0   │100% │ 1:34 PM");
}

/// The reported case: a deep path on an 80-column terminal kept its head and
/// lost the name of the file being viewed.
#[test]
fn a_deep_path_at_80_columns_still_shows_the_file_name() {
    let viewer = file_viewer("/srv/x/agents/v-viewer-7/work/docs/medium.txt", b"text");
    let row = bar_row(&viewer, Cells::new(80), CLOCK, 23);
    assert!(
        row.starts_with("…") && row.contains("/docs/medium.txt│t│UTF-8│4 "),
        "{row:?}"
    );
    assert_eq!(Cells::of(&row).get(), 80);
}

/// A report has no path to cut: its title keeps its head.
#[test]
fn a_narrow_report_bar_keeps_the_head_of_its_title() {
    let viewer = Viewer::report("MCP Server Status", Vec::new());
    let row = bar_row(&viewer, Cells::new(15), CLOCK, 23);
    assert_eq!(row, "MCP S…│ 1:34 PM");
}

#[test]
fn a_bar_too_narrow_for_the_fields_is_cut_as_a_whole() {
    let viewer = file_viewer("/tmp/x/main.rs", b"fn main() {}");
    let row = bar_row(&viewer, Cells::new(30), CLOCK, 23);
    assert!(row.starts_with("│t│UTF-8│12"), "{row:?}");
    assert_eq!(Cells::of(&row).get(), 30);
    assert_eq!(bar_row(&viewer, Cells::ZERO, CLOCK, 23), "");
}

#[test]
fn a_cjk_path_is_measured_in_cells_and_never_panics() {
    let viewer = file_viewer("/tmp/日本語/ファイル名.txt", b"x");
    for width in 0..60 {
        let row = bar_row(&viewer, Cells::new(width), CLOCK, 23);
        assert_eq!(Cells::of(&row).get(), width, "{row:?}");
    }
}

#[test]
fn a_report_shows_its_title_and_only_the_clock() {
    let viewer = Viewer::report("MCP Server Status", vec!["Tools:".to_owned()]);
    let row = bar_row(&viewer, Cells::new(40), CLOCK, 23);
    assert_eq!(row, format!("MCP Server Status{}│ 1:34 PM", " ".repeat(14)));
}

// ── The whole screen ─────────────────────────────────────────────────────

#[test]
fn the_status_line_fills_row_zero_in_far_colours() {
    let mut app = app_with(file_viewer("/tmp/x/main.rs", b"fn main() {}"));
    let terminal = draw(&mut app, 80, 24);
    let row = row_text(&terminal, 0);
    assert!(row.starts_with("/tmp/x/main.rs"), "{row:?}");
    assert!(row.contains("│t│UTF-8│12      │Col 0   │100% │"), "{row:?}");
    assert!(row.ends_with(" AM") || row.ends_with(" PM"), "{row:?}");
    let buf = terminal.backend().buffer();
    for x in 0..80 {
        assert_eq!(buf[(x, 0)].bg, palette::STATUS_BAR_BG, "column {x}");
        assert_eq!(buf[(x, 0)].fg, palette::STATUS_BAR_FG, "column {x}");
    }
}
