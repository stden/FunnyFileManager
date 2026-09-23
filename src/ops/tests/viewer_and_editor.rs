//! Viewer, editor and modal operation regression tests.

use crate::scroll::ScrollTop;

use super::support::*;
use crate::ops::*;
use crate::popup::process_popup_input;

#[cfg(feature = "test-helpers")]
use crate::editor::{handle_edit_test_mode, TestEditorBehavior};

use crate::models::{App, Overlay, Panel, Viewer, Viewport};

use crossterm::event::KeyCode;

use std::fs::{self, File};

use std::io::{self, Write};

use tempfile::TempDir;

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

#[test]
fn test_handle_view_file() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let file_path = temp_dir.path().join("test.txt");
    File::create(&file_path)
        .expect("test setup")
        .write_all(b"Line 1\nLine 2\nLine 3")
        .expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    // Select the file
    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == "test.txt" {
            app.left_panel.select(Some(i));
            break;
        }
    }

    handle_view(&mut app).expect("test setup");

    assert!(app.get_viewer().is_some());
    let viewer = app.get_viewer().unwrap();
    assert_eq!(viewer.title(), "test.txt");
    assert_eq!(viewer.lines.len(), 3);
    assert_eq!(viewer.lines[0], "Line 1");
    assert!(app.status_text().contains("Viewing"));

    Ok(())
}

#[test]
fn test_handle_view_empty_file() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let file_path = temp_dir.path().join("empty.txt");
    File::create(&file_path).expect("test setup"); // Empty file

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    // Select the file
    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == "empty.txt" {
            app.left_panel.select(Some(i));
            break;
        }
    }

    handle_view(&mut app).expect("test setup");

    assert!(app.get_viewer().is_some());
    let viewer = app.get_viewer().unwrap();
    assert_eq!(viewer.lines.len(), 1);
    assert_eq!(viewer.lines[0], "<empty file>");

    Ok(())
}

#[test]
fn test_handle_view_invalid_utf8() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let file_path = temp_dir.path().join("bad.txt");
    fs::write(&file_path, b"\xFF").expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == "bad.txt" {
            app.left_panel.select(Some(i));
            break;
        }
    }

    // Non-UTF-8 *text* is read in the editor's single-byte code page rather
    // than failing the whole action.
    handle_view(&mut app).expect("a non-UTF-8 decode must not fail");
    let viewer = app.get_viewer().expect("viewer must open");
    assert_eq!(viewer.lines, vec!["\u{44f}".to_owned()]);

    Ok(())
}

#[test]
fn test_handle_view_directory() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    fs::create_dir(temp_dir.path().join("subdir")).expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    // Select the directory
    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == "subdir" {
            app.left_panel.select(Some(i));
            break;
        }
    }

    handle_view(&mut app).expect("test setup");

    assert!(app.get_viewer().is_none());
    assert!(
        matches!(app.overlay, Overlay::Information(_)),
        "F3 on a directory measures it, as Far does"
    );

    Ok(())
}

#[test]
fn test_handle_view_no_selection() -> io::Result<()> {
    let mut app = App::new().expect("test setup");
    app.left_panel.select(None);

    // Should not crash
    handle_view(&mut app).expect("test setup");
    assert!(app.get_viewer().is_none());

    Ok(())
}

#[test]
#[cfg(feature = "test-helpers")]
fn test_handle_edit_success() -> io::Result<()> {
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

    handle_edit_test_mode(&mut app, TestEditorBehavior::Success).expect("test setup");
    assert!(app.status_text().contains("Edited"));

    Ok(())
}

#[test]
#[cfg(feature = "test-helpers")]
fn test_handle_edit_fail_launch() -> io::Result<()> {
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

    handle_edit_test_mode(&mut app, TestEditorBehavior::FailLaunch).expect("test setup");
    assert!(app.status_text().contains("Failed to launch editor"));

    Ok(())
}

#[test]
#[cfg(feature = "test-helpers")]
fn test_handle_edit_exit_error() -> io::Result<()> {
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

    handle_edit_test_mode(&mut app, TestEditorBehavior::ExitError).expect("test setup");
    assert!(app.status_text().contains("Editor exited with status"));

    Ok(())
}

#[test]
#[cfg(feature = "test-helpers")]
fn test_handle_edit_directory() -> io::Result<()> {
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

    handle_edit_test_mode(&mut app, TestEditorBehavior::Success).expect("test setup");
    assert_eq!(app.status_text(), "Editing directories is not supported");

    Ok(())
}
