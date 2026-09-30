//! Unit tests: Far's full-screen viewer body — where it starts, paging and
//! panning, clipping, the terminal's height, highlighting, and the panels
//! it covers — rendered through the real `ui` path.

use super::tests::{app_with, draw, file_viewer, row_foregrounds, row_text};
use super::*;
use crate::scroll::Scrolled;

#[test]
fn the_body_starts_at_column_zero_on_row_one_and_is_highlighted() {
    let mut app = app_with(file_viewer("/tmp/x/main.rs", b"fn main() {}"));
    let terminal = draw(&mut app, 80, 24);
    assert_eq!(row_text(&terminal, 1), "fn main() {}");
    let buf = terminal.backend().buffer();
    assert_eq!(buf[(0, 1)].fg, palette::CODE_KEYWORD, "`fn` is a keyword");
    assert_eq!(buf[(0, 1)].bg, palette::BACKGROUND);
    assert_ne!(buf[(0, 1)].fg, palette::CODE_NORMAL);
}

#[test]
fn a_report_body_is_plain_and_its_bar_has_no_file_fields() {
    let mut app = app_with(Viewer::report(
        "MCP Server Status",
        vec!["fn is not a keyword here".to_owned()],
    ));
    let terminal = draw(&mut app, 80, 24);
    let bar = row_text(&terminal, 0);
    assert!(bar.starts_with("MCP Server Status"), "{bar:?}");
    assert!(!bar.contains("│t│"), "{bar:?}");
    assert_eq!(bar.matches('│').count(), 1, "{bar:?}");
    assert_eq!(row_text(&terminal, 1), "fn is not a keyword here");
    for fg in row_foregrounds(&terminal, 1) {
        assert_eq!(fg, palette::CODE_NORMAL);
    }
}

#[test]
fn the_keys_page_from_the_viewport() {
    use crate::models::Viewport;
    let mut app = app_with(file_viewer("/tmp/x/a.txt", b"a\nb"));
    let _terminal = draw(&mut app, 80, 10);
    assert_eq!(
        Viewer::page_for(app.viewport),
        23,
        "drawing does not write the body height back into the model"
    );
    app.set_viewport(Viewport::new(10));
    assert_eq!(Viewer::page_for(app.viewport), 9);
}

#[test]
fn a_taller_terminal_re_anchors_the_last_line_on_the_bottom_row() {
    let text = (0..100)
        .map(|i| i.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    let mut app = app_with(file_viewer("/tmp/x/a.txt", text.as_bytes()));
    app.set_viewport(crate::models::Viewport::new(24));
    let page = Viewer::page_for(app.viewport);
    app.get_viewer_mut().expect("viewer").scroll_to_bottom(page);
    assert_eq!(app.get_viewer().expect("viewer").scroll.lines(), 77);

    app.set_viewport(crate::models::Viewport::new(50));
    let terminal = draw(&mut app, 80, 50);
    assert_eq!(app.get_viewer().expect("viewer").scroll.lines(), 51);
    assert_eq!(row_text(&terminal, 1), "51");
    assert_eq!(
        row_text(&terminal, 49),
        "99",
        "no blank rows under the file"
    );
    assert!(row_text(&terminal, 0).contains("│100% │"));
}

#[test]
fn panning_right_shifts_the_body_and_the_col_field() {
    let mut app = app_with(file_viewer("/tmp/x/a.txt", b"abcdef"));
    app.get_viewer_mut().expect("viewer").scroll_right(1);
    let terminal = draw(&mut app, 80, 24);
    assert_eq!(row_text(&terminal, 1), "bcdef");
    assert!(row_text(&terminal, 0).contains("│Col 1   │"));
}

#[test]
fn a_long_line_is_clipped_at_the_right_edge_not_wrapped() {
    let mut app = app_with(file_viewer("/tmp/x/a.txt", b"x".repeat(200).as_slice()));
    let terminal = draw(&mut app, 40, 5);
    assert_eq!(row_text(&terminal, 1), "x".repeat(40));
    assert_eq!(row_text(&terminal, 2), "");
}

#[test]
fn a_one_row_terminal_shows_only_the_bar_and_does_not_panic() {
    let mut app = app_with(file_viewer("/tmp/x/a.txt", b"a\nb"));
    let terminal = draw(&mut app, 80, 1);
    assert!(row_text(&terminal, 0).contains("│t│UTF-8│"));
    let _terminal = draw(&mut app, 80, 0);
    let _terminal = draw(&mut app, 0, 0);
}

#[test]
fn a_markdown_heading_is_not_plain_text() {
    let mut app = app_with(file_viewer("/tmp/x/README.md", b"# Title\nbody"));
    let terminal = draw(&mut app, 80, 24);
    let buf = terminal.backend().buffer();
    assert_ne!(
        buf[(0, 1)].fg,
        palette::CODE_NORMAL,
        "a heading is coloured, not painted as body text"
    );
    assert_eq!(row_text(&terminal, 1), "# Title");
    assert_eq!(row_text(&terminal, 2), "body");
}

#[test]
fn a_gitignore_comment_is_coloured_as_a_comment() {
    let mut app = app_with(file_viewer(
        "/tmp/x/.gitignore",
        b"# Byte-compiled\n__pycache__/\n",
    ));
    let terminal = draw(&mut app, 80, 24);
    let buf = terminal.backend().buffer();
    assert_eq!(
        buf[(0, 1)].fg,
        palette::CODE_COMMENT,
        ".gitignore comments start with #, the same as a shell file"
    );
    assert_ne!(buf[(0, 2)].fg, palette::CODE_COMMENT);
}

#[test]
fn nothing_of_the_panels_survives_underneath() {
    let mut app = app_with(file_viewer("/tmp/x/a.txt", b"hi"));
    let terminal = draw(&mut app, 80, 24);
    let buf = terminal.backend().buffer();
    for y in 2..24 {
        assert_eq!(row_text(&terminal, y), "", "row {y}");
        assert_eq!(buf[(40, y)].bg, palette::BACKGROUND, "row {y}");
    }
}
