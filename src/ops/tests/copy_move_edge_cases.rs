//! Unit tests: copy move edge cases (split out to keep files under 500 lines).

use crate::models::FileName;
use crate::models::Lifecycle;

use super::support::*;
use crate::ops::*;
use crate::popup::process_popup_input;

use crate::models::{App, CommandLine, Overlay, PanelSide, Viewer};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use std::fs::{self, File};

use std::io::{self, Write};

#[cfg(not(coverage))]
use std::path::{Path, PathBuf};

use tempfile::TempDir;

/// A destination well past Windows' historical 260-character `MAX_PATH` must
/// either receive the file or say why. The one outcome a file manager may not
/// produce is a "Copied to …" with no file under it.
#[test]
#[cfg(not(coverage))]
fn test_handle_copy_move_with_very_long_paths() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let mut app = App::new().expect("test setup");

    let long_path = long_destination(temp_dir.path());

    let source_file = temp_dir.path().join("source.txt");
    File::create(&source_file)
        .expect("test setup")
        .write_all(b"content")
        .expect("test setup");

    app.left_panel.path = temp_dir.path().to_path_buf();
    app.right_panel.path = long_path.clone();
    app.left_panel.load_entries().expect("test setup");
    app.right_panel.load_entries().expect("test setup");
    // By name, not by index: directories sort first, so index 1 is the nested
    // directory the destination lives *inside*, and copying that only ever
    // exercised the copy-into-itself guard — never a long path.
    assert!(app.left_panel.select_named("source.txt"));

    // Per-file failures are folded into the status summary, so the call itself
    // succeeds whichever way the filesystem went.
    handle_copy(&mut app).expect("handle_copy reports per-file errors in its status");

    let copied = long_path.join("source.txt");
    if copied.exists() {
        assert_eq!(fs::read_to_string(&copied)?, "content");
        assert!(
            app.status_text().contains("Copied to"),
            "a copy that landed must be reported as one: {}",
            app.status_text()
        );
    } else {
        assert!(
            app.status_text().contains("error"),
            "a copy that did not land must say so, not report success: {}",
            app.status_text()
        );
    }

    Ok(())
}

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
fn test_handle_copy_to_same_directory() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let file_path = temp_dir.path().join("file.txt");
    File::create(&file_path).expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel.path = temp_dir.path().to_path_buf();
    app.right_panel.path = temp_dir.path().to_path_buf();
    app.left_panel.load_entries().expect("test setup");

    // Select file.txt (skip ..)
    app.left_panel.select(Some(1));

    handle_copy(&mut app).expect("test setup");

    // Should create a unique name like file_1.txt
    assert!(temp_dir.path().join("file_1.txt").exists());
    assert!(app.status_text().contains("Copied to"));

    Ok(())
}

#[test]
fn test_handle_move_to_same_directory() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let file_path = temp_dir.path().join("move_me.txt");
    File::create(&file_path).expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel.path = temp_dir.path().to_path_buf();
    app.right_panel.path = temp_dir.path().to_path_buf();
    app.left_panel.load_entries().expect("test setup");

    // Select move_me.txt
    app.left_panel.select(Some(1));

    handle_move(&mut app).expect("test setup");

    // Should be renamed to move_me_1.txt
    assert!(!file_path.exists());
    assert!(temp_dir.path().join("move_me_1.txt").exists());
    assert!(app.status_text().contains("Moved to"));

    Ok(())
}

#[test]
fn test_handle_copy_marked_to_different_dir() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let left_dir = temp_dir.path().join("left");
    let right_dir = temp_dir.path().join("right");
    fs::create_dir(&left_dir).expect("test setup");
    fs::create_dir(&right_dir).expect("test setup");

    let f1 = left_dir.join("f1.txt");
    let f2 = left_dir.join("f2.txt");
    File::create(&f1).expect("test setup");
    File::create(&f2).expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel.path = left_dir;
    app.right_panel.path = right_dir.clone();
    app.left_panel.load_entries().expect("test setup");

    app.left_panel.marked.insert(FileName::from("f1.txt"));
    app.left_panel.marked.insert(FileName::from("f2.txt"));

    handle_copy(&mut app).expect("test setup");

    assert!(right_dir.join("f1.txt").exists());
    assert!(right_dir.join("f2.txt").exists());
    assert!(app.status_text().contains("Copied 2 item(s)"));

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

#[test]
fn test_process_popup_input_drive_selection() -> io::Result<()> {
    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::DriveSelection {
        side: PanelSide::Left,
        selected_index: 0,
        rows: crate::models::drive_rows(),
    });

    process_popup_input(&mut app, KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    process_popup_input(&mut app, KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
    process_popup_input(&mut app, KeyEvent::new(KeyCode::End, KeyModifiers::NONE));
    process_popup_input(&mut app, KeyEvent::new(KeyCode::Home, KeyModifiers::NONE));
    if let Overlay::DriveSelection { selected_index, .. } = &app.overlay {
        assert_eq!(*selected_index, 0);
    }

    process_popup_input(&mut app, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert!(!matches!(app.overlay, Overlay::DriveSelection { .. }));

    app.set_overlay(Overlay::DriveSelection {
        side: PanelSide::Left,
        selected_index: 0,
        rows: crate::models::drive_rows(),
    });
    process_popup_input(&mut app, key_event(KeyCode::Enter));
    assert!(!matches!(app.overlay, Overlay::DriveSelection { .. }));

    Ok(())
}

#[test]
fn test_process_popup_input_delete_confirm() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let file_path = temp_dir.path().join("to_delete.txt");
    File::create(&file_path).expect("test setup");

    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::DeleteConfirm(vec![file_path.clone()]));

    process_popup_input(
        &mut app,
        KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE),
    );
    assert!(!matches!(app.overlay, Overlay::DeleteConfirm(_)));
    assert!(file_path.exists());

    app.set_overlay(Overlay::DeleteConfirm(vec![file_path.clone()]));
    process_popup_input(
        &mut app,
        KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE),
    );
    assert!(!matches!(app.overlay, Overlay::DeleteConfirm(_)));
    assert!(!file_path.exists());

    Ok(())
}

#[test]
fn test_process_popup_input_command_line_trigger() -> io::Result<()> {
    let mut app = App::new().expect("test setup");

    // Simulating the ':' key being pressed in the main loop logic
    app.set_overlay(Overlay::CommandLine(CommandLine::default()));

    // Now test interaction with the command line overlay
    process_popup_input(
        &mut app,
        KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE),
    );
    process_popup_input(
        &mut app,
        KeyEvent::new(KeyCode::Char('w'), KeyModifiers::NONE),
    );
    process_popup_input(
        &mut app,
        KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE),
    );

    if let Overlay::CommandLine(cmd) = &app.overlay {
        assert_eq!(cmd.text(), "pwd");
    }

    // Enter should execute and close
    process_popup_input(&mut app, key_event(KeyCode::Enter));
    assert!(!matches!(app.overlay, Overlay::CommandLine(_)));

    Ok(())
}

#[test]
fn test_process_popup_input_new_file_input() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let mut app = App::new().expect("test setup");
    app.left_panel.path = temp_dir.path().to_path_buf();
    app.set_overlay(Overlay::NewFileInput(String::new()));

    for character in "new.txt".chars() {
        process_popup_input(&mut app, key_event(KeyCode::Char(character)));
    }

    if let Overlay::NewFileInput(input) = &app.overlay {
        assert_eq!(input, "new.txt");
    }

    process_popup_input(&mut app, key_event(KeyCode::Backspace));
    if let Overlay::NewFileInput(input) = &app.overlay {
        assert_eq!(input, "new.tx");
    }
    process_popup_input(&mut app, key_event(KeyCode::Char('t')));

    process_popup_input(&mut app, key_event(KeyCode::Enter));
    assert!(!matches!(app.overlay, Overlay::NewFileInput(_)));
    assert!(temp_dir.path().join("new.txt").exists());

    // Test empty filename
    app.set_overlay(Overlay::NewFileInput("  ".to_owned()));
    process_popup_input(&mut app, key_event(KeyCode::Enter));
    assert!(!matches!(app.overlay, Overlay::NewFileInput(_)));
    assert!(app.status_text().contains("cannot be empty"));

    // Test existing file
    app.set_overlay(Overlay::NewFileInput("new.txt".to_owned()));
    process_popup_input(&mut app, key_event(KeyCode::Enter));
    assert!(!matches!(app.overlay, Overlay::NewFileInput(_)));
    assert!(app.status_text().contains("already exists"));

    Ok(())
}

#[test]
fn test_execute_command_pwd_quit_exit() -> io::Result<()> {
    let mut app = App::new().expect("test setup");

    execute_command(&mut app, "pwd").expect("test setup");
    assert!(!app.status_text().is_empty());

    execute_command(&mut app, "quit").expect("test setup");
    assert!(!app.lifecycle.is_running());

    app.lifecycle = Lifecycle::Running;
    execute_command(&mut app, "exit").expect("test setup");
    assert!(!app.lifecycle.is_running());

    Ok(())
}

#[cfg(not(coverage))]
fn long_destination(root: &Path) -> PathBuf {
    let mut long_path = root.to_path_buf();
    for i in 0..10 {
        long_path = long_path.join(format!("very_long_directory_name_{i}"));
        fs::create_dir(&long_path).expect("test setup");
    }
    assert!(
        long_path.to_string_lossy().len() > 260,
        "the destination has to clear MAX_PATH for this to be testing anything"
    );

    long_path
}
