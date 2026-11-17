//! Unit tests: an active quick-view panel scrolls with the viewer's keys, as
//! the app's keymap gives them, and leaves every other key to the file-panel
//! actions.

use std::fs;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tempfile::TempDir;

use super::process;
use crate::action::viewer::ViewerCommand;
use crate::action::Bindings;
use crate::keymap::tables::VIEWER;
use crate::keymap::{ActionName, Explicit};
use crate::models::{App, Panel, PanelKind, Viewport};
use crate::popup::tests::key;
use crate::test_support::chord;

fn app_over_a_long_file() -> (TempDir, App) {
    let dir = TempDir::new().expect("temp dir");
    let body: Vec<String> = (1..=40).map(|n| format!("line {n}")).collect();
    fs::write(dir.path().join("long.txt"), body.join("\n")).expect("write");
    let panel = Panel::new(dir.path().to_path_buf()).expect("panel");
    let mut app = App::with_panels(panel.clone(), panel);
    app.set_viewport(Viewport::new(24));
    let index = app
        .left_panel
        .entries
        .iter()
        .position(|entry| entry.name == "long.txt");
    app.left_panel.select(index);
    crate::ops::quickview::toggle(&mut app);
    app.switch_panel();
    assert!(app.get_active_panel_ref().kind.is_quick_view());
    (dir, app)
}

/// The viewer's keys with `action` on `text` alone.
fn remapped(action: &'static str, text: &str) -> Bindings<ViewerCommand> {
    let explicit = Explicit::from([(ActionName::new(action), vec![chord(text)])]);
    VIEWER.layered(&explicit).bindings
}

fn scroll(app: &App) -> (usize, usize) {
    match &app.right_panel.kind {
        PanelKind::QuickView(view) => (view.scroll.lines(), view.col.cells()),
        other => panic!("not a quick view: {other:?}"),
    }
}

#[test]
fn the_arrows_scroll_the_preview() {
    let (_dir, mut app) = app_over_a_long_file();
    assert!(process(&mut app, key(KeyCode::Down)));
    assert!(process(&mut app, key(KeyCode::Down)));
    assert!(process(&mut app, key(KeyCode::Up)));
    assert!(process(&mut app, key(KeyCode::Right)));
    assert_eq!(scroll(&app), (1, 1));
    assert!(process(&mut app, key(KeyCode::Left)));
    assert!(process(&mut app, key(KeyCode::Left)));
    assert_eq!(scroll(&app), (1, 0));
}

#[test]
fn paging_moves_by_the_panel_body_and_end_stops_on_the_last_line() {
    let (_dir, mut app) = app_over_a_long_file();
    assert!(process(&mut app, key(KeyCode::PageDown)));
    assert_eq!(scroll(&app), (17, 0));
    assert!(process(&mut app, key(KeyCode::End)));
    assert_eq!(scroll(&app), (23, 0));
    assert!(process(&mut app, key(KeyCode::PageUp)));
    assert_eq!(scroll(&app), (6, 0));
    assert!(process(&mut app, key(KeyCode::Home)));
    assert_eq!(scroll(&app), (0, 0));
}

#[test]
fn ctrl_arrows_pan_twenty_columns() {
    let (_dir, mut app) = app_over_a_long_file();
    let ctrl_right = KeyEvent::new(KeyCode::Right, KeyModifiers::CONTROL);
    let ctrl_left = KeyEvent::new(KeyCode::Left, KeyModifiers::CONTROL);
    assert!(process(&mut app, ctrl_right));
    assert!(process(&mut app, ctrl_right));
    assert_eq!(scroll(&app), (0, 40));
    assert!(process(&mut app, ctrl_left));
    assert_eq!(scroll(&app), (0, 20));
}

#[test]
fn every_other_key_falls_through_to_the_panel_actions() {
    let (_dir, mut app) = app_over_a_long_file();
    assert!(!process(&mut app, key(KeyCode::Tab)));
    assert!(!process(&mut app, key(KeyCode::F(10))));
    assert!(!process(
        &mut app,
        KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL)
    ));
    assert_eq!(scroll(&app), (0, 0));
}

#[test]
fn a_file_panel_is_never_scrolled_here() {
    let dir = TempDir::new().expect("temp dir");
    let panel = Panel::new(dir.path().to_path_buf()).expect("panel");
    let mut app = App::with_panels(panel.clone(), panel);
    assert!(!process(&mut app, key(KeyCode::Down)));
}

/// Exact matching: a scroll key with a modifier nobody bound is not the
/// preview's, so it goes on to the file-panel actions.
#[test]
fn a_scroll_key_with_a_modifier_nobody_bound_falls_through() {
    let (_dir, mut app) = app_over_a_long_file();
    for (code, modifiers) in [
        (KeyCode::Down, KeyModifiers::SHIFT),
        (KeyCode::PageDown, KeyModifiers::CONTROL),
        (KeyCode::End, KeyModifiers::ALT),
        (KeyCode::Right, KeyModifiers::SHIFT),
        (KeyCode::Right, KeyModifiers::CONTROL | KeyModifiers::SHIFT),
    ] {
        let pressed = KeyEvent::new(code, modifiers);
        assert!(!process(&mut app, pressed), "{pressed:?}");
    }
    assert_eq!(scroll(&app), (0, 0));
}

#[test]
fn the_preview_obeys_the_scroll_keys_of_the_apps_keymap() {
    let (_dir, mut app) = app_over_a_long_file();
    app.keymap.viewer = remapped("line_down", "j");
    assert!(process(&mut app, key(KeyCode::Char('j'))));
    assert_eq!(scroll(&app), (1, 0));
    assert!(
        !process(&mut app, key(KeyCode::Down)),
        "the panels' key now"
    );
    assert_eq!(scroll(&app), (1, 0));
}

/// The viewer's way out is never the preview's, whichever key it has.
#[test]
fn a_remapped_closing_key_falls_through_too() {
    let (_dir, mut app) = app_over_a_long_file();
    app.keymap.viewer = remapped("close", "q");
    assert!(!process(&mut app, key(KeyCode::Char('q'))));
    assert!(app.get_active_panel_ref().kind.is_quick_view());
}
