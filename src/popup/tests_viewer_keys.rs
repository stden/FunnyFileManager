//! Unit tests: Far's viewer keys, against a viewer that has been drawn once.

use super::process_popup_input;
use super::tests::key;
use crate::models::{App, Overlay, Viewer, Viewport};
use crate::scroll::{ScrollLeft, ScrollTop};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// A hundred-line viewer whose keys page as if the body were `rows` tall.
fn drawn_viewer(rows: usize) -> App {
    let mut app = App::new().expect("test app");
    let lines = (0..100).map(|i| format!("line {i}")).collect();
    app.set_overlay(Overlay::Viewer(Viewer::report("big.txt", lines)));
    let height = u16::try_from(rows.saturating_add(1)).expect("test height");
    app.set_viewport(Viewport::new(height));
    app
}

fn ctrl(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::CONTROL)
}

fn scroll(app: &App) -> usize {
    app.get_viewer().expect("viewer open").scroll.lines()
}

fn col(app: &App) -> ScrollLeft {
    app.get_viewer().expect("viewer open").col
}

#[test]
fn down_stops_with_the_last_line_on_the_bottom_row() {
    let mut app = drawn_viewer(23);
    for _ in 0..500 {
        assert!(process_popup_input(&mut app, key(KeyCode::Down)));
    }
    assert_eq!(scroll(&app), 77, "lines 77..=99 fill the 23 rows");
}

#[test]
fn page_down_and_page_up_move_by_the_body_height() {
    let mut app = drawn_viewer(23);
    process_popup_input(&mut app, key(KeyCode::PageDown));
    assert_eq!(scroll(&app), 23);
    process_popup_input(&mut app, key(KeyCode::PageDown));
    assert_eq!(scroll(&app), 46);
    process_popup_input(&mut app, key(KeyCode::PageUp));
    assert_eq!(scroll(&app), 23);
    process_popup_input(&mut app, key(KeyCode::PageUp));
    process_popup_input(&mut app, key(KeyCode::PageUp));
    assert_eq!(
        app.get_viewer().expect("viewer open").scroll,
        ScrollTop::TOP
    );
}

#[test]
fn page_down_is_clamped_like_down() {
    let mut app = drawn_viewer(23);
    for _ in 0..10 {
        process_popup_input(&mut app, key(KeyCode::PageDown));
    }
    assert_eq!(scroll(&app), 77);
}

#[test]
fn end_shows_the_last_screen_and_home_the_first() {
    let mut app = drawn_viewer(23);
    process_popup_input(&mut app, key(KeyCode::End));
    assert_eq!(scroll(&app), 77);
    process_popup_input(&mut app, key(KeyCode::Home));
    assert_eq!(
        app.get_viewer().expect("viewer open").scroll,
        ScrollTop::TOP
    );
}

#[test]
fn left_and_right_pan_one_cell_and_left_stops_at_the_edge() {
    let mut app = drawn_viewer(23);
    assert!(process_popup_input(&mut app, key(KeyCode::Left)));
    assert_eq!(col(&app), ScrollLeft::LEFT);
    process_popup_input(&mut app, key(KeyCode::Right));
    process_popup_input(&mut app, key(KeyCode::Right));
    assert_eq!(col(&app).cells(), 2);
    process_popup_input(&mut app, key(KeyCode::Left));
    assert_eq!(col(&app).cells(), 1);
}

#[test]
fn ctrl_left_and_ctrl_right_pan_twenty_cells() {
    let mut app = drawn_viewer(23);
    process_popup_input(&mut app, ctrl(KeyCode::Right));
    assert_eq!(col(&app).cells(), 20);
    process_popup_input(&mut app, key(KeyCode::Right));
    process_popup_input(&mut app, ctrl(KeyCode::Left));
    assert_eq!(col(&app).cells(), 1);
    process_popup_input(&mut app, ctrl(KeyCode::Left));
    assert_eq!(col(&app), ScrollLeft::LEFT);
}

#[test]
fn a_file_that_fits_never_scrolls() {
    let mut app = App::new().expect("test app");
    app.set_overlay(Overlay::Viewer(Viewer::report(
        "short.txt",
        vec!["a".to_owned(), "b".to_owned()],
    )));
    for code in [KeyCode::Down, KeyCode::PageDown, KeyCode::End] {
        process_popup_input(&mut app, key(code));
        assert_eq!(
            app.get_viewer().expect("viewer open").scroll,
            ScrollTop::TOP
        );
    }
}
