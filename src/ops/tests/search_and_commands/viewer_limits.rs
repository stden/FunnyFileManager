//! Purpose: The viewer's navigation and scroll stop at the ends.

use super::*;

#[test]
fn test_process_popup_input_viewer_navigation_limits() -> io::Result<()> {
    let mut app = App::new().expect("test setup");
    let lines: Vec<String> = (0..20).map(|i| format!("Line {i}")).collect();

    app.set_overlay(Overlay::Viewer(Viewer::report("Test", lines)));

    for _ in 0..30 {
        process_popup_input(&mut app, key_event(KeyCode::Up));
    }
    assert_eq!(app.get_viewer().unwrap().scroll, ScrollTop::TOP);

    for _ in 0..100 {
        process_popup_input(&mut app, key_event(KeyCode::Down));
    }

    let viewer = app.get_viewer().expect("viewer");
    assert!(viewer.scroll.lines() <= viewer.lines.len());

    Ok(())
}

#[test]
fn test_process_popup_input_viewer_scroll_limits() -> io::Result<()> {
    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::Viewer(Viewer::report(
        "Test",
        vec!["Line 1".to_owned(), "Line 2".to_owned()],
    )));
    app.set_viewport(crate::models::Viewport::new(2));

    // Holding Down stops at the last line rather than running past it (one
    // body row, so the last line may reach the top).
    // This used to force `scroll` to 1000 and assert it stayed there: the old
    // `if scroll < max` guard skipped the increment but left the bad value
    // alone. `ScrollTop::down` clamps, so an offset past the end is pulled
    // back — and production can no longer produce one, since every call passes
    // the real last index.
    for _ in 0..50 {
        process_popup_input(
            &mut app,
            KeyEvent::new(KeyCode::Down, KeyModifiers::empty()),
        );
    }
    assert_eq!(
        app.get_viewer().unwrap().scroll.lines(),
        1,
        "two lines, so the last one is index 1"
    );

    process_popup_input(
        &mut app,
        KeyEvent::new(KeyCode::PageDown, KeyModifiers::empty()),
    );
    assert_eq!(app.get_viewer().unwrap().scroll.lines(), 1);

    Ok(())
}
