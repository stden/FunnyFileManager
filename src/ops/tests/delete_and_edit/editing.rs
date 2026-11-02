//! Purpose: F4 through the editor seam: success, an error exit, a launch failure, nothing and a folder.

use super::*;

#[test]
#[cfg(feature = "test-helpers")]
fn test_handle_edit_no_selection() -> io::Result<()> {
    let mut app = App::new().expect("test setup");
    app.left_panel.select(None);

    // Should not crash
    handle_edit_test_mode(&mut app, TestEditorBehavior::Success).expect("test setup");
    Ok(())
}

#[test]
#[cfg(feature = "test-helpers")]
fn test_handle_edit_impl_success() -> io::Result<()> {
    use ratatui::Terminal;

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("test setup");

    let temp_dir = TempDir::new().expect("test setup");
    let file_path = temp_dir.path().join("test.txt");
    File::create(&file_path).expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    // Select the file
    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == "test.txt" {
            app.left_panel.select(Some(i));
            break;
        }
    }

    // Mock successful editor run
    let result = handle_edit_impl(&mut terminal, &mut app, |editor, _args, path| {
        let _ = editor;
        let _ = path;
        Ok(fake_exit_status(0))
    });

    assert!(result.is_ok());
    assert!(app.status_text().contains("Edited"));

    Ok(())
}

#[test]
#[cfg(feature = "test-helpers")]
fn test_handle_edit_impl_editor_error_exit() -> io::Result<()> {
    use ratatui::Terminal;

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("test setup");

    let temp_dir = TempDir::new().expect("test setup");
    let file_path = temp_dir.path().join("test.txt");
    File::create(&file_path).expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    // Select the file
    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == "test.txt" {
            app.left_panel.select(Some(i));
            break;
        }
    }

    // Mock editor with error exit code
    let result = handle_edit_impl(&mut terminal, &mut app, |editor, _args, path| {
        let _ = editor;
        let _ = path;
        Ok(fake_exit_status(1))
    });

    assert!(result.is_ok());
    assert!(app.status_text().contains("Editor exited with status"));

    Ok(())
}

#[test]
#[cfg(feature = "test-helpers")]
fn test_handle_edit_impl_launch_failure() -> io::Result<()> {
    use ratatui::Terminal;

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("test setup");

    let temp_dir = TempDir::new().expect("test setup");
    let file_path = temp_dir.path().join("test.txt");
    File::create(&file_path).expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    // Select the file
    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == "test.txt" {
            app.left_panel.select(Some(i));
            break;
        }
    }

    // Mock editor launch failure
    let result = handle_edit_impl(&mut terminal, &mut app, |editor, _args, path| {
        let _ = editor;
        let _ = path;
        Err(io::Error::new(io::ErrorKind::NotFound, "Editor not found"))
    });

    assert!(result.is_ok());
    assert!(app.status_text().contains("Failed to launch editor"));

    Ok(())
}

#[test]
#[cfg(feature = "test-helpers")]
fn test_handle_edit_impl_no_selection() -> io::Result<()> {
    let mut app = App::new().expect("test setup");
    app.left_panel.select(None);
    let initial_status = app.status_text();

    let result = handle_edit_test_mode(&mut app, TestEditorBehavior::Success);

    assert!(result.is_ok());
    assert_eq!(app.status_text(), initial_status);

    Ok(())
}

#[test]
#[cfg(feature = "test-helpers")]
fn test_handle_edit_impl_directory() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    fs::create_dir(temp_dir.path().join("subdir")).expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    // Select the directory
    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == "subdir" {
            app.left_panel.select(Some(i));
            break;
        }
    }

    let result = handle_edit_test_mode(&mut app, TestEditorBehavior::Success);

    assert!(result.is_ok());
    assert_eq!(app.status_text(), "Editing directories is not supported");

    Ok(())
}
