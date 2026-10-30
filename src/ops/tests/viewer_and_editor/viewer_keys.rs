//! Purpose: Viewer and help keys: scrolling, its bounds, and closing.

use super::*;

#[test]
fn test_process_popup_input_viewer() -> io::Result<()> {
    let mut app = App::new().expect("test setup");

    // Create a viewer with multiple lines
    app.set_overlay(Overlay::Viewer(Viewer::report(
        "test.txt",
        vec![
            "Line 1".to_owned(),
            "Line 2".to_owned(),
            "Line 3".to_owned(),
            "Line 4".to_owned(),
            "Line 5".to_owned(),
        ],
    )));
    // One body row, so Down walks the file instead of being swallowed by a
    // 23-row default page that already shows all five lines.
    app.set_viewport(Viewport::new(2));

    // Test scrolling down
    assert!(process_popup_input(&mut app, key_event(KeyCode::Down)));
    assert_eq!(app.get_viewer().unwrap().scroll.lines(), 1);

    // Test scrolling up
    assert!(process_popup_input(&mut app, key_event(KeyCode::Up)));
    assert_eq!(app.get_viewer().unwrap().scroll, ScrollTop::TOP);

    // Test scrolling up at top (should stay at 0)
    assert!(process_popup_input(&mut app, key_event(KeyCode::Up)));
    assert_eq!(app.get_viewer().unwrap().scroll, ScrollTop::TOP);

    // Test closing viewer with Esc
    assert!(process_popup_input(&mut app, key_event(KeyCode::Esc)));
    assert!(app.get_viewer().is_none());

    Ok(())
}

#[test]
fn test_process_popup_input_viewer_close_variations() -> io::Result<()> {
    let mut app = App::new().expect("test setup");

    // Test closing with F3
    app.set_overlay(Overlay::Viewer(Viewer::report(
        "test.txt",
        vec!["Line 1".to_owned()],
    )));
    assert!(process_popup_input(&mut app, key_event(KeyCode::F(3))));
    assert!(app.get_viewer().is_none());

    // Test closing with Enter
    app.set_overlay(Overlay::Viewer(Viewer::report(
        "test.txt",
        vec!["Line 1".to_owned()],
    )));
    assert!(process_popup_input(&mut app, key_event(KeyCode::Enter)));
    assert!(app.get_viewer().is_none());

    Ok(())
}

#[test]
fn test_process_popup_input_viewer_max_scroll() -> io::Result<()> {
    let mut app = App::new().expect("test setup");

    // Create a viewer with limited lines
    app.set_overlay(Overlay::Viewer(Viewer::report(
        "test.txt",
        vec!["Line 1".to_owned(), "Line 2".to_owned()],
    )));

    // Try to scroll beyond available lines
    for _ in 0..10 {
        process_popup_input(&mut app, key_event(KeyCode::Down));
    }
    // Should stop at maximum scroll position
    assert!(app.get_viewer().unwrap().scroll.lines() <= 1);

    Ok(())
}

#[test]
fn test_viewer_scroll_exact_bounds() -> io::Result<()> {
    let mut app = App::new().expect("test setup");

    // Single line: scroll should stay at 0
    app.set_overlay(Overlay::Viewer(Viewer::report(
        "test.txt",
        vec!["only line".to_owned()],
    )));
    process_popup_input(&mut app, key_event(KeyCode::Down));
    assert_eq!(app.get_viewer().unwrap().scroll, ScrollTop::TOP);

    // Empty lines: scroll should stay at 0
    app.set_overlay(Overlay::Viewer(Viewer::report("test.txt", vec![])));
    process_popup_input(&mut app, key_event(KeyCode::Down));
    assert_eq!(app.get_viewer().unwrap().scroll, ScrollTop::TOP);

    // 3 lines on a one-line page: the last line may reach the top row, so
    // the max scroll is 2
    app.set_overlay(Overlay::Viewer(Viewer::report(
        "test.txt",
        vec!["a".into(), "b".into(), "c".into()],
    )));
    app.set_viewport(Viewport::new(2));
    for _ in 0..10 {
        process_popup_input(&mut app, key_event(KeyCode::Down));
    }
    assert_eq!(app.get_viewer().unwrap().scroll.lines(), 2);

    Ok(())
}

#[test]
fn test_process_popup_input_help() -> io::Result<()> {
    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::Help);

    // Test closing help with Esc
    assert!(process_popup_input(&mut app, key_event(KeyCode::Esc)));
    assert!(!matches!(app.overlay, Overlay::Help));

    app.set_overlay(Overlay::Help);
    // Test closing help with F1
    assert!(process_popup_input(&mut app, key_event(KeyCode::F(1))));
    assert!(!matches!(app.overlay, Overlay::Help));

    Ok(())
}

#[test]
fn test_process_popup_input_no_overlay() -> io::Result<()> {
    let mut app = App::new().expect("test setup");

    // No viewer or help shown - should return false
    assert!(!process_popup_input(&mut app, key_event(KeyCode::Down)));
    assert!(!process_popup_input(&mut app, key_event(KeyCode::Up)));
    assert!(!process_popup_input(&mut app, key_event(KeyCode::Esc)));

    Ok(())
}
