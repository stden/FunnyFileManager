//! Purpose: Pins how a viewer is built from a file's bytes, how many rows a
//! terminal gives it, and where it stops scrolling.

use std::path::PathBuf;

use super::*;
use crate::scroll::{ScrollLeft, ScrollTop};

fn file(bytes: &[u8]) -> Viewer {
    Viewer::file(PathBuf::from("/tmp/notes.txt"), bytes)
}

/// The file behind a viewer, or a failed test if it is a report.
fn viewed(viewer: &Viewer) -> &ViewedFile {
    match &viewer.source {
        ViewSource::File(file) => file,
        ViewSource::Report(title) => panic!("a file viewer, not the report {title:?}"),
    }
}

/// A numbered report `lines` long.
fn numbered(lines: usize) -> Viewer {
    Viewer::report("t", (0..lines).map(|i| i.to_string()).collect())
}

#[test]
fn a_utf8_file_is_labelled_utf8_and_split_into_lines() {
    let viewer = file("hello\nwörld\n".as_bytes());
    assert_eq!(viewer.lines, vec!["hello", "wörld"]);
    let file = viewed(&viewer);
    assert_eq!(file.encoding, Encoding::Utf8);
    assert_eq!(file.encoding.label(), "UTF-8");
    assert_eq!(file.size.bytes(), 13);
}

#[test]
fn bytes_that_are_not_utf8_open_lossily_as_ansi() {
    let viewer = file(b"caf\xe9\nok");
    assert_eq!(viewer.lines, vec!["caf\u{fffd}", "ok"]);
    let file = viewed(&viewer);
    assert_eq!(file.encoding, Encoding::Ansi);
    assert_eq!(file.encoding.label(), "ANSI");
    assert_eq!(file.size.bytes(), 7);
}

#[test]
fn an_empty_file_shows_the_placeholder_and_a_zero_size() {
    let viewer = file(b"");
    assert_eq!(viewer.lines, vec!["<empty file>"]);
    let file = viewed(&viewer);
    assert_eq!(file.size.bytes(), 0);
    assert_eq!(file.encoding, Encoding::Utf8);
}

#[test]
fn a_file_viewer_keeps_the_path_and_opens_at_the_top_left() {
    let viewer = file(b"x");
    assert_eq!(viewer.scroll, ScrollTop::TOP);
    assert_eq!(viewer.col, ScrollLeft::LEFT);
    assert_eq!(viewed(&viewer).path, PathBuf::from("/tmp/notes.txt"));
}

#[test]
fn a_report_is_named_by_its_title_and_keeps_its_lines_verbatim() {
    let viewer = Viewer::report("MCP Server Status", Vec::new());
    assert_eq!(
        viewer.source,
        ViewSource::Report("MCP Server Status".to_owned())
    );
    assert!(viewer.lines.is_empty(), "a report says <no data> itself");
    assert_eq!(viewer.scroll, ScrollTop::TOP);
}

#[test]
fn the_page_is_the_terminal_less_the_status_line_and_never_zero() {
    assert_eq!(Viewer::page_for(Viewport::new(24)), 23);
    assert_eq!(Viewer::page_for(Viewport::default()), 23);
    assert_eq!(Viewer::page_for(Viewport::new(1)), 1);
    assert_eq!(Viewer::page_for(Viewport::new(0)), 1);
}

#[test]
fn scrolling_stops_when_the_last_line_reaches_the_bottom_row() {
    let mut viewer = numbered(10);
    viewer.scroll_down(100, 4);
    assert_eq!(viewer.scroll.lines(), 6, "lines 6..=9 fill the four rows");
    viewer.scroll_up(2);
    assert_eq!(viewer.scroll.lines(), 4);
}

#[test]
fn a_taller_page_pulls_the_view_back_to_keep_the_last_line_on_the_bottom_row() {
    let mut viewer = numbered(100);
    viewer.scroll_to_bottom(23);
    assert_eq!(viewer.scroll.lines(), 77);
    viewer.clamp(49);
    assert_eq!(viewer.scroll.lines(), 51, "lines 51..=99 fill the 49 rows");
    viewer.scroll_up(1);
    assert_eq!(viewer.scroll.lines(), 50);
}

#[test]
fn a_shorter_page_leaves_the_top_row_where_it_was() {
    let mut viewer = numbered(100);
    viewer.scroll_down(51, 49);
    viewer.clamp(23);
    assert_eq!(
        viewer.scroll.lines(),
        51,
        "the last line merely leaves the screen"
    );
}

#[test]
fn end_lands_on_the_last_screen_and_home_returns_to_the_top() {
    let mut viewer = numbered(10);
    viewer.scroll_right(3);
    viewer.scroll_to_bottom(4);
    assert_eq!(viewer.scroll.lines(), 6);
    assert_eq!(viewer.col, ScrollLeft::LEFT, "End returns to the left edge");
    viewer.scroll_right(3);
    viewer.scroll_to_top();
    assert_eq!(viewer.scroll, ScrollTop::TOP);
    assert_eq!(
        viewer.col,
        ScrollLeft::LEFT,
        "Home returns to the left edge"
    );
}

#[test]
fn a_file_shorter_than_the_screen_never_scrolls() {
    let mut viewer = numbered(3);
    viewer.scroll_down(1, 23);
    viewer.scroll_to_bottom(23);
    assert_eq!(viewer.scroll, ScrollTop::TOP);
}

#[test]
fn horizontal_scroll_saturates_at_the_left_edge() {
    let mut viewer = numbered(1);
    viewer.scroll_left(1);
    assert_eq!(viewer.col, ScrollLeft::LEFT);
    viewer.scroll_right(20);
    viewer.scroll_right(1);
    assert_eq!(viewer.col.cells(), 21);
    viewer.scroll_left(20);
    assert_eq!(viewer.col.cells(), 1);
}
