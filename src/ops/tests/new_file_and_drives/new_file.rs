//! Purpose: Shift+F4 and F7: a new file, an existing name, and the first new folder.

use super::*;

// -----------------------------------------------------------------------
// New-file input overlay -- Enter with already-existing file sets error msg
// but keeps the new_file_input set to None (the overlay is cleared)
// -----------------------------------------------------------------------

#[test]
fn test_new_file_input_enter_existing_file_sets_error_message() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let existing = temp_dir.path().join("exists.txt");
    File::create(&existing).expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.left_panel.load_entries().expect("test setup");
    app.set_overlay(Overlay::NewFileInput("exists.txt".to_owned()));

    process_popup_input(&mut app, key_event(KeyCode::Enter));

    // Overlay is cleared.
    assert!(
        app.get_new_file_input_mut().is_none(),
        "overlay should be cleared"
    );
    assert!(
        app.status_text().contains("already exists"),
        "status should note that file already exists"
    );

    Ok(())
}

#[test]
fn test_handle_mkdir_first_time() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    handle_mkdir(&mut app).expect("test setup");

    assert!(temp_dir.path().join("new_folder").exists());
    assert!(app.status_text().contains("Created directory"));

    Ok(())
}

#[test]
fn test_new_file_input_enter_creates_new_file() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.left_panel.load_entries().expect("test setup");
    app.set_overlay(Overlay::NewFileInput("brand_new.txt".to_owned()));

    process_popup_input(&mut app, key_event(KeyCode::Enter));

    assert!(
        !matches!(app.overlay, Overlay::NewFileInput(_)),
        "overlay should be cleared after creation"
    );
    assert!(
        temp_dir.path().join("brand_new.txt").exists(),
        "file should have been created"
    );
    assert!(
        app.status_text().contains("Created file"),
        "status should confirm creation"
    );

    Ok(())
}
