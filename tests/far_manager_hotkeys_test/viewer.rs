//! Purpose: F3 viewer: opening, closing, scrolling, paging and panning.

use super::*;

// ─── F3 VIEW ────────────────────────────────────────────────────────────────

#[test]
fn hotkey_f3_opens_viewer_for_file() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("readme.txt"), b"hello world").unwrap();
    let mut app = setup(&temp).unwrap();
    let idx = app
        .left_panel
        .entries
        .iter()
        .position(|e| e.name == "readme.txt")
        .unwrap();
    app.left_panel.select(Some(idx));
    handle_view(&mut app).unwrap();
    assert!(app.get_viewer().is_some(), "viewer should be set after F3");
    assert_eq!(app.get_viewer().unwrap().title(), "readme.txt");
}

#[test]
fn hotkey_f3_no_selection_is_noop() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    app.left_panel.select(None);
    handle_view(&mut app).unwrap();
    assert!(app.get_viewer().is_none());
}

#[test]
fn overlay_viewer_esc_closes() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    app.set_overlay(Overlay::Viewer(Viewer::report("t", vec!["line".into()])));
    process_popup_input(&mut app, press(KeyCode::Esc));
    assert!(app.get_viewer().is_none());
}

#[test]
fn overlay_viewer_f3_closes() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    app.set_overlay(Overlay::Viewer(Viewer::report("t", vec!["line".into()])));
    process_popup_input(&mut app, press(KeyCode::F(3)));
    assert!(app.get_viewer().is_none());
}

#[test]
fn overlay_viewer_enter_closes() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    app.set_overlay(Overlay::Viewer(Viewer::report("t", vec!["line".into()])));
    process_popup_input(&mut app, press(KeyCode::Enter));
    assert!(app.get_viewer().is_none());
}

#[test]
fn overlay_viewer_down_scrolls() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    let lines: Vec<String> = (0..20).map(|i| format!("line {i}")).collect();
    app.set_overlay(Overlay::Viewer(Viewer::report("t", lines)));
    app.set_viewport(funny_file_manager::models::Viewport::new(2));
    process_popup_input(&mut app, press(KeyCode::Down));
    assert_eq!(app.get_viewer().unwrap().scroll.lines(), 1);
}

#[test]
fn overlay_viewer_up_at_zero_stays_at_zero() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    app.set_overlay(Overlay::Viewer(Viewer::report("t", vec!["line".into()])));
    process_popup_input(&mut app, press(KeyCode::Up));
    assert_eq!(app.get_viewer().unwrap().scroll, ScrollTop::TOP);
}

#[test]
fn overlay_viewer_up_decrements_scroll() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    let lines: Vec<String> = (0..20).map(|i| format!("line {i}")).collect();
    app.set_overlay(Overlay::Viewer(Viewer {
        scroll: ScrollTop::TOP.down(5, usize::MAX),
        ..Viewer::report("t", lines)
    }));
    process_popup_input(&mut app, press(KeyCode::Up));
    assert_eq!(app.get_viewer().unwrap().scroll.lines(), 4);
}

/// A hundred-line viewer as it stands after one frame with 23 body rows.
fn drawn_viewer(temp: &TempDir) -> io::Result<App> {
    let mut app = setup(temp)?;
    let lines: Vec<String> = (0..100).map(|i| format!("line {i}")).collect();
    app.set_overlay(Overlay::Viewer(Viewer::report("t", lines)));
    app.set_viewport(funny_file_manager::models::Viewport::new(24));
    Ok(app)
}

#[test]
fn overlay_viewer_down_stops_with_the_last_line_on_the_bottom_row() {
    let temp = TempDir::new().unwrap();
    let mut app = drawn_viewer(&temp).unwrap();
    for _ in 0..200 {
        process_popup_input(&mut app, press(KeyCode::Down));
    }
    assert_eq!(app.get_viewer().unwrap().scroll.lines(), 77);
}

#[test]
fn overlay_viewer_pgdn_pgup_move_one_screen() {
    let temp = TempDir::new().unwrap();
    let mut app = drawn_viewer(&temp).unwrap();
    process_popup_input(&mut app, press(KeyCode::PageDown));
    assert_eq!(app.get_viewer().unwrap().scroll.lines(), 23);
    process_popup_input(&mut app, press(KeyCode::PageUp));
    assert_eq!(app.get_viewer().unwrap().scroll, ScrollTop::TOP);
}

#[test]
fn overlay_viewer_end_and_home_jump_to_the_last_and_first_screen() {
    let temp = TempDir::new().unwrap();
    let mut app = drawn_viewer(&temp).unwrap();
    process_popup_input(&mut app, press(KeyCode::End));
    assert_eq!(app.get_viewer().unwrap().scroll.lines(), 77);
    process_popup_input(&mut app, press(KeyCode::Home));
    assert_eq!(app.get_viewer().unwrap().scroll, ScrollTop::TOP);
}

#[test]
fn overlay_viewer_left_right_pan_one_column_and_ctrl_twenty() {
    let temp = TempDir::new().unwrap();
    let mut app = drawn_viewer(&temp).unwrap();
    process_popup_input(&mut app, press(KeyCode::Left));
    assert_eq!(app.get_viewer().unwrap().col.cells(), 0, "Left at 0 stays");
    process_popup_input(&mut app, press(KeyCode::Right));
    assert_eq!(app.get_viewer().unwrap().col.cells(), 1);
    process_popup_input(&mut app, press_mod(KeyCode::Right, KeyModifiers::CONTROL));
    assert_eq!(app.get_viewer().unwrap().col.cells(), 21);
    process_popup_input(&mut app, press_mod(KeyCode::Left, KeyModifiers::CONTROL));
    assert_eq!(app.get_viewer().unwrap().col.cells(), 1);
    process_popup_input(&mut app, press(KeyCode::Left));
    assert_eq!(app.get_viewer().unwrap().col.cells(), 0);
}
