//! Purpose: Pins the terminal height the model paging reads, and what storing
//! a new one does to an open viewer.

use super::*;
use crate::models::{App, Overlay, Viewer};

/// A hundred-line report open in a fresh app.
fn app_with_long_viewer() -> App {
    let mut app = App::new().expect("test app");
    let lines = (0..100).map(|i| i.to_string()).collect();
    app.set_overlay(Overlay::Viewer(Viewer::report("t", lines)));
    app
}

fn top(app: &App) -> usize {
    app.get_viewer().expect("viewer open").scroll.lines()
}

#[test]
fn a_fresh_app_assumes_the_classic_screen() {
    let app = App::new().expect("test app");
    assert_eq!(app.viewport, Viewport::default());
    assert_eq!(app.viewport.rows(), FALLBACK_TERMINAL_HEIGHT);
    assert_eq!(FALLBACK_TERMINAL_HEIGHT, 24);
}

#[test]
fn set_viewport_stores_the_height() {
    let mut app = App::new().expect("test app");
    app.set_viewport(Viewport::new(50));
    assert_eq!(app.viewport.rows(), 50);
    app.set_viewport(Viewport::new(0));
    assert_eq!(app.viewport, Viewport::new(0));
}

#[test]
fn a_taller_terminal_pulls_an_open_viewer_back_to_keep_the_last_line_on_the_bottom_row() {
    let mut app = app_with_long_viewer();
    let page = Viewer::page_for(app.viewport);
    app.get_viewer_mut()
        .expect("viewer open")
        .scroll_to_bottom(page);
    assert_eq!(top(&app), 77, "lines 77..=99 fill the 23 rows");

    app.set_viewport(Viewport::new(50));
    assert_eq!(top(&app), 51, "lines 51..=99 fill the 49 rows");
}

#[test]
fn a_shorter_terminal_leaves_the_top_row_where_it_was() {
    let mut app = app_with_long_viewer();
    app.set_viewport(Viewport::new(50));
    let page = Viewer::page_for(app.viewport);
    app.get_viewer_mut()
        .expect("viewer open")
        .scroll_to_bottom(page);
    assert_eq!(top(&app), 51);

    app.set_viewport(Viewport::new(24));
    assert_eq!(top(&app), 51, "the last line merely leaves the screen");
}

#[test]
fn set_viewport_touches_nothing_when_no_viewer_is_open() {
    let mut app = App::new().expect("test app");
    app.set_overlay(Overlay::Help);
    app.set_viewport(Viewport::new(50));
    assert!(matches!(app.overlay, Overlay::Help));
    assert_eq!(app.viewport.rows(), 50);
}
