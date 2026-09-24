//! Purpose: F8 of a file and a folder, F5/F6 with nothing selected, and keys the viewer and help ignore.

use super::*;

#[test]
fn test_handle_delete_file() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let file_path = temp_dir.path().join("delete.txt");
    File::create(&file_path).expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == "delete.txt" {
            app.left_panel.select(Some(i));
            break;
        }
    }

    handle_delete_confirm(&mut app).expect("test setup");
    handle_delete(&mut app).expect("test setup");

    assert!(!file_path.exists());
    assert!(app.status_text().contains("Deleted"));

    Ok(())
}

#[test]
fn test_handle_delete_directory() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let dir_path = temp_dir.path().join("delete_dir");
    fs::create_dir(&dir_path).expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == "delete_dir" {
            app.left_panel.select(Some(i));
            break;
        }
    }

    handle_delete_confirm(&mut app).expect("test setup");
    handle_delete(&mut app).expect("test setup");

    assert!(!dir_path.exists());
    assert!(app.status_text().contains("Deleted"));

    Ok(())
}

#[test]
fn test_handle_copy_no_selection() -> io::Result<()> {
    let mut app = App::new().expect("test setup");
    app.left_panel.select(None);

    // Should not crash
    handle_copy(&mut app).expect("test setup");

    Ok(())
}

#[test]
fn test_handle_move_no_selection() -> io::Result<()> {
    let mut app = App::new().expect("test setup");
    app.left_panel.select(None);

    // Should not crash
    handle_move(&mut app).expect("test setup");

    Ok(())
}

#[test]
fn test_process_popup_input_viewer_other_key() -> io::Result<()> {
    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::Viewer(Viewer::report(
        "test.txt",
        vec!["Line 1".to_owned()],
    )));

    // Test with a key that doesn't do anything in viewer
    assert!(process_popup_input(&mut app, key_event(KeyCode::Char('a'))));
    // Viewer should still be active
    assert!(app.get_viewer().is_some());

    Ok(())
}

#[test]
fn test_process_popup_input_help_other_key() -> io::Result<()> {
    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::Help);

    // Test with a key that doesn't close help
    assert!(process_popup_input(&mut app, key_event(KeyCode::Down)));
    // Help should still be shown
    assert!(matches!(app.overlay, Overlay::Help));

    Ok(())
}
