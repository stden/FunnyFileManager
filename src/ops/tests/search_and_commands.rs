//! Unit tests: search and commands (split out to keep files under 500 lines).

use crate::scroll::ScrollTop;

use super::support::*;
use crate::ops::*;
use crate::popup::process_popup_input;

use crate::models::{App, CommandLine, Overlay, Viewer};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use std::fs::{self, File};

use std::io::{self, Write};

use tempfile::TempDir;

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
fn test_execute_command_edge_cases() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let mut app = App::new().expect("test setup");
    app.left_panel.path = temp_dir.path().to_path_buf();

    assert!(execute_command(&mut app, "").is_err());
    assert!(execute_command(&mut app, "   ").is_err());
    // "unknown_command" is dispatched to the shell; the process runs (Ok)
    // even though the command itself is not found by the shell.
    assert!(execute_command(&mut app, "unknown_command").is_ok());

    execute_command(&mut app, "  help  ").expect("test setup");
    assert!(matches!(app.overlay, Overlay::Help));
    app.close_overlay();

    // "HELP" (uppercase) is not a recognized built-in, so it runs as a
    // shell command and returns Ok.
    assert!(execute_command(&mut app, "HELP").is_ok());

    Ok(())
}

#[test]
fn test_execute_command_cd_missing_argument() -> io::Result<()> {
    let mut app = App::new().expect("test setup");

    let result = execute_command(&mut app, "cd");
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().kind(), io::ErrorKind::InvalidInput);

    Ok(())
}

#[test]
fn test_execute_command_cd_invalid_path() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let mut app = App::new().expect("test setup");
    app.left_panel.path = temp_dir.path().to_path_buf();

    // Try to cd to a file instead of directory
    let file = temp_dir.path().join("file.txt");
    File::create(&file).expect("test setup");

    let result = execute_command(&mut app, &format!("cd {}", file.display()));
    assert!(result.is_err());

    Ok(())
}

#[test]
fn test_execute_command_sort_invalid_mode() -> io::Result<()> {
    let mut app = App::new().expect("test setup");

    let result = execute_command(&mut app, "sort invalid");
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().kind(), io::ErrorKind::InvalidInput);

    Ok(())
}

#[test]
fn test_execute_command_bookmark_missing_subcommand() -> io::Result<()> {
    let mut app = App::new().expect("test setup");

    let result = execute_command(&mut app, "bookmark");
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().kind(), io::ErrorKind::InvalidInput);

    Ok(())
}

#[test]
fn test_execute_command_bookmark_invalid_slot() -> io::Result<()> {
    let mut app = App::new().expect("test setup");

    // With BookmarkSlot + Command::Invalid, bad slots produce errors (exact message may vary).
    assert!(execute_command(&mut app, "bookmark set 10").is_err());
    assert!(execute_command(&mut app, "bookmark set abc").is_err());

    Ok(())
}

#[test]
fn test_execute_command_bookmark_go_empty() -> io::Result<()> {
    let mut app = App::new().expect("test setup");

    // Try to go to an empty bookmark
    let result = execute_command(&mut app, "bookmark go 0");
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("Bookmark") || err_msg.contains("empty"));

    Ok(())
}

#[test]
fn test_execute_command_bookmark_invalid_subcommand() -> io::Result<()> {
    let mut app = App::new().expect("test setup");

    let result = execute_command(&mut app, "bookmark invalid 0");
    assert!(result.is_err());
    // Just check that it returns an error
    assert!(result.is_err());

    Ok(())
}

#[test]
fn test_execute_command_mark_missing_argument() -> io::Result<()> {
    let mut app = App::new().expect("test setup");

    let result = execute_command(&mut app, "mark");
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().kind(), io::ErrorKind::InvalidInput);

    Ok(())
}

#[test]
fn test_execute_command_mark_invalid_subcommand() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let mut app = App::new().expect("test setup");
    app.left_panel.path = temp_dir.path().to_path_buf();
    app.left_panel.load_entries().expect("test setup");

    let result = execute_command(&mut app, "mark invalid");
    assert!(result.is_err());
    // Just check that it returns an error
    assert!(result.is_err());

    Ok(())
}

#[test]
fn test_execute_command_unknown_command() -> io::Result<()> {
    let mut app = App::new().expect("test setup");

    // Unknown commands are dispatched to the shell, which returns Ok
    // (the shell process itself runs successfully even if the command
    // is not found).
    let result = execute_command(&mut app, "unknowncommand arg1 arg2");
    assert!(result.is_ok());
    // The status_message should reflect the shell error output
    assert!(!app.status_text().is_empty());

    Ok(())
}

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
