//! Unit tests: delete and edit (split out to keep files under 500 lines).
#![allow(unused_imports)]

use super::support::*;
use crate::ops::*;
use crate::popup::process_popup_input;

#[cfg(feature = "test-helpers")]
use crate::editor::{handle_edit_impl, handle_edit_test_mode, TestEditorBehavior};

use crate::models::{App, Overlay, Panel, Viewer};

#[cfg(feature = "test-helpers")]
use crate::WritableTestBackend;

use crossterm::event::KeyCode;

use std::fs::{self, File};

use std::io::{self};

use tempfile::TempDir;

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
        vec!["Line 1".to_string()],
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
