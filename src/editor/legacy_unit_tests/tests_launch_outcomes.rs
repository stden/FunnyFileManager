//! Purpose: Feature-gated tests of how `handle_edit_impl` reports editor launch failures
//! and non-zero exit codes.
use super::editor_tests::create_test_app;
use crate::editor::handle_edit_impl;
use crate::models::App;
use ratatui::Terminal;
use std::fs::File;
use std::io;
use std::io::Write;
use tempfile::TempDir;

#[test]
fn test_handle_edit_impl_editor_error() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("test.txt");
    File::create(&file_path)?;

    let mut app = create_test_app(&temp_dir)?;

    // Find and select the file
    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == "test.txt" {
            app.left_panel.select(Some(i));
            break;
        }
    }

    let backend = crate::WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend)?;

    let result = handle_edit_impl(&mut terminal, &mut app, |editor, args, path| {
        let _ = editor;
        let _ = args;
        let _ = path;
        Err(io::Error::new(io::ErrorKind::NotFound, "Editor not found"))
    });

    assert!(result.is_ok());
    assert!(app.status_text().contains("Failed to launch editor"));
    assert!(app.status_text().contains("Editor not found"));
    Ok(())
}

#[test]
fn test_handle_edit_impl_editor_exit_code() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("test.txt");
    File::create(&file_path)?;

    let mut app = create_test_app(&temp_dir)?;

    // Find and select the file
    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == "test.txt" {
            app.left_panel.select(Some(i));
            break;
        }
    }

    let backend = crate::WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend)?;

    // Create a mock ExitStatus with error code
    #[cfg(unix)]
    let exit_status = std::os::unix::process::ExitStatusExt::from_raw(256); // Exit code 1
    #[cfg(windows)]
    let exit_status = std::process::ExitStatus::default(); // Windows doesn't have from_raw

    let result = handle_edit_impl(&mut terminal, &mut app, |_, _, _| Ok(exit_status));

    assert!(result.is_ok());
    #[cfg(unix)]
    assert!(app.status_text().contains("Editor exited with status"));
    Ok(())
}

#[test]
fn test_handle_edit_impl_terminal_error() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;

    let file = temp_dir.path().join("test.txt");
    File::create(&file)?.write_all(b"content")?;

    app.left_panel.path = temp_dir.path().to_path_buf();
    app.left_panel.load_entries()?;
    app.left_panel.select(Some(1));

    let backend = crate::WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend)?;

    // Test with editor that returns error status
    let result = handle_edit_impl(&mut terminal, &mut app, |_editor, _args, _path| {
        Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "Cannot execute editor",
        ))
    });

    assert!(result.is_ok());
    assert!(app.status_text().contains("Failed to launch editor"));
    Ok(())
}

#[test]
fn test_handle_edit_impl_exit_error() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("test.txt");
    File::create(&file_path)?;

    let mut app = create_test_app(&temp_dir)?;

    // Find and select the file
    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == "test.txt" {
            app.left_panel.select(Some(i));
            break;
        }
    }

    let backend = crate::WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend)?;

    // Test that load_entries error is handled
    let result = handle_edit_impl(&mut terminal, &mut app, |editor, args, path| {
        let _ = editor;
        let _ = args;
        let _ = path;
        Ok(std::process::ExitStatus::default())
    });

    // The error from load_entries would be propagated
    assert!(result.is_ok());
    Ok(())
}

#[test]
fn test_handle_edit_impl_launch_error() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("test.txt");
    File::create(&file_path)?;

    let mut app = create_test_app(&temp_dir)?;

    // Find and select the file
    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == "test.txt" {
            app.left_panel.select(Some(i));
            break;
        }
    }

    let backend = crate::WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend)?;

    let result = handle_edit_impl(&mut terminal, &mut app, |editor, args, path| {
        let _ = editor;
        let _ = args;
        let _ = path;
        Err(io::Error::other("Launch failed"))
    });

    assert!(result.is_ok());
    assert!(app.status_text().contains("Failed to launch editor"));
    assert!(app.status_text().contains("Launch failed"));
    Ok(())
}
