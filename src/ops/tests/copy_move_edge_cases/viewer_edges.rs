//! Purpose: The viewer overlay stays itself, and F3 over an open viewer.

use super::*;

#[test]
fn test_process_popup_input_viewer_always_some() -> io::Result<()> {
    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::Viewer(Viewer::report(
        "Test",
        vec!["Line 1".to_owned(), "Line 2".to_owned()],
    )));

    // Test various viewer operations to ensure viewer remains Some
    process_popup_input(&mut app, key_event(KeyCode::Down));
    assert!(app.get_viewer().is_some());

    process_popup_input(&mut app, key_event(KeyCode::Up));
    assert!(app.get_viewer().is_some());

    // Unknown keys don't close viewer
    process_popup_input(&mut app, key_event(KeyCode::Char('x')));
    assert!(app.get_viewer().is_some());

    Ok(())
}

#[test]
fn test_handle_view_with_viewer_already_open() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let mut app = App::new().expect("test setup");

    // Create a file to view
    let file_path = temp_dir.path().join("view.txt");
    File::create(&file_path)
        .expect("test setup")
        .write_all(b"content")
        .expect("test setup");

    app.left_panel.path = temp_dir.path().to_path_buf();
    app.left_panel.load_entries().expect("test setup");
    app.left_panel.select(Some(1));

    // Open viewer first time
    handle_view(&mut app).expect("test setup");
    assert!(app.get_viewer().is_some());

    // Try to open viewer again (should replace)
    handle_view(&mut app).expect("test setup");
    assert!(app.get_viewer().is_some());

    Ok(())
}
