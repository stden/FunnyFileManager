//! Purpose: F3 on an unreadable folder, a failing command line, and a cross-device move.

use super::*;

#[test]
fn test_handle_view_directory_error() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let mut app = App::new().expect("test setup");

    let dir = temp_dir.path().join("subdir");
    fs::create_dir(&dir).expect("test setup");

    app.left_panel.path = temp_dir.path().to_path_buf();
    app.left_panel.load_entries().expect("test setup");

    // Find and select the subdir
    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == "subdir" && entry.kind.is_dir() {
            app.left_panel.select(Some(i));
            break;
        }
    }

    // handle_view returns Ok for directories but shows a message
    let result = handle_view(&mut app);
    assert!(result.is_ok());
    assert!(app.status_text().contains("Cannot view directory") || app.get_viewer().is_none());

    Ok(())
}

#[test]
fn test_process_popup_input_command_execute_error() -> io::Result<()> {
    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::CommandLine(CommandLine::default()));
    app.set_overlay(Overlay::CommandLine("unknown_command".into()));

    process_popup_input(
        &mut app,
        KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()),
    );
    assert!(!matches!(app.overlay, Overlay::CommandLine(_)));
    assert!(app.status_text().contains("Error") || app.status_text().contains("unknown"));

    Ok(())
}

#[test]
fn test_handle_move_cross_device_fallback() -> io::Result<()> {
    // This test simulates cross-device move by using the force rename failure
    let temp_dir = TempDir::new().expect("test setup");
    let mut app = App::new().expect("test setup");

    let source_file = temp_dir.path().join("source.txt");
    File::create(&source_file)
        .expect("test setup")
        .write_all(b"content")
        .expect("test setup");

    let dest_dir = temp_dir.path().join("dest");
    fs::create_dir(&dest_dir).expect("test setup");

    app.left_panel.path = temp_dir.path().to_path_buf();
    app.right_panel.path = dest_dir;
    app.left_panel.load_entries().expect("test setup");
    app.right_panel.load_entries().expect("test setup");

    // Find source.txt in entries
    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == "source.txt" {
            app.left_panel.select(Some(i));
            break;
        }
    }

    // Force rename to fail to trigger fallback
    let guard = ForceRenameFailureGuard::enable();

    let result = handle_move(&mut app);

    drop(guard);

    assert!(result.is_ok());
    assert!(!source_file.exists()); // Source should be deleted

    Ok(())
}
