//! Unit tests: Far's viewer keys, against a viewer that has been drawn once —
//! the default keys exactly as written, and the keys the app's keymap gives.

use super::process_popup_input;
use super::tests::key;
use crate::keymap::tables::VIEWER;
use crate::keymap::{ActionName, Explicit};
use crate::models::{App, Overlay, Viewer, Viewport};
use crate::scroll::{ScrollLeft, ScrollTop};
use crate::test_support::chord;
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

/// Exact matching: the viewer swallows a key with a modifier nobody bound
/// and neither scrolls nor closes on it.
#[test]
fn a_key_with_a_modifier_nobody_bound_does_nothing() {
    let mut app = drawn_viewer(23);
    for (code, modifiers) in [
        (KeyCode::Down, KeyModifiers::SHIFT),
        (KeyCode::PageDown, KeyModifiers::CONTROL),
        (KeyCode::End, KeyModifiers::ALT),
        (KeyCode::Right, KeyModifiers::SHIFT),
        (KeyCode::Right, KeyModifiers::CONTROL | KeyModifiers::SHIFT),
        (KeyCode::Esc, KeyModifiers::ALT),
        (KeyCode::Enter, KeyModifiers::CONTROL),
        (KeyCode::F(3), KeyModifiers::SHIFT),
        (KeyCode::F(10), KeyModifiers::ALT),
    ] {
        let pressed = KeyEvent::new(code, modifiers);
        assert!(process_popup_input(&mut app, pressed), "{pressed:?}");
        assert_eq!(scroll(&app), 0, "{pressed:?}");
        assert_eq!(col(&app), ScrollLeft::LEFT, "{pressed:?}");
    }
}

#[test]
fn the_viewer_obeys_the_keys_of_the_apps_keymap() {
    let mut app = drawn_viewer(23);
    let explicit = Explicit::from([
        (ActionName::new("line_down"), vec![chord("j")]),
        (ActionName::new("close"), vec![chord("q")]),
    ]);
    app.keymap.viewer = VIEWER.layered(&explicit).bindings;
    process_popup_input(&mut app, key(KeyCode::Char('j')));
    assert_eq!(scroll(&app), 1);
    assert!(process_popup_input(&mut app, key(KeyCode::Down)));
    assert_eq!(scroll(&app), 1, "Down has no command left");
    assert!(process_popup_input(&mut app, key(KeyCode::Esc)));
    assert!(app.get_viewer().is_some(), "Esc has no command left");
    process_popup_input(&mut app, key(KeyCode::Char('q')));
    assert!(app.get_viewer().is_none());
}
