//! Unit tests and shared test helpers for `handlers`.
#![allow(unused_imports)]

use super::file_ops::{
    format_operation_status, get_source_paths, rename_or_fail, FORCE_RENAME_FAILURE,
    RENAME_TEST_MUTEX,
};
use crate::scroll::ScrollTop;

use super::shell::{run_shell_command, windows_shell_args};

use super::*;

use crate::command::Command;

#[cfg(feature = "test-helpers")]
use crate::editor::{handle_edit_impl, handle_edit_test_mode, TestEditorBehavior};

use crate::models::{
    App, BookmarkSlot, Overlay, Panel, PanelSide, SortMode, Viewer, MAX_BOOKMARKS,
};

#[cfg(feature = "test-helpers")]
use crate::WritableTestBackend;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use std::fs::{self, File};

use std::io::{self, Write};

use std::path::{Path, PathBuf};

use std::sync::atomic::Ordering;

use tempfile::TempDir;

#[cfg(feature = "test-helpers")]
pub fn fake_exit_status(code: i32) -> std::process::ExitStatus {
    use std::process::Command;
    if code == 0 {
        Command::new("true").status().unwrap()
    } else {
        Command::new("false").status().unwrap()
    }
}

pub struct ForceRenameFailureGuard {
    _lock: std::sync::MutexGuard<'static, ()>,
}

impl ForceRenameFailureGuard {
    pub(crate) fn enable() -> Self {
        let lock = RENAME_TEST_MUTEX
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        FORCE_RENAME_FAILURE.store(true, Ordering::SeqCst);
        Self { _lock: lock }
    }
}

impl Drop for ForceRenameFailureGuard {
    fn drop(&mut self) {
        FORCE_RENAME_FAILURE.store(false, Ordering::SeqCst);
    }
}

pub fn key_event(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::empty())
}

pub fn key_event_with_mods(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
    KeyEvent::new(code, modifiers)
}

#[test]
fn test_process_popup_input_viewer() -> io::Result<()> {
    let mut app = App::new().expect("test setup");

    // Create a viewer with multiple lines
    app.set_overlay(Overlay::Viewer(Viewer {
        title: "test.txt".to_string(),
        lines: vec![
            "Line 1".to_string(),
            "Line 2".to_string(),
            "Line 3".to_string(),
            "Line 4".to_string(),
            "Line 5".to_string(),
        ],
        scroll: ScrollTop::TOP,
    }));

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
    app.set_overlay(Overlay::Viewer(Viewer {
        title: "test.txt".to_string(),
        lines: vec!["Line 1".to_string()],
        scroll: ScrollTop::TOP,
    }));
    assert!(process_popup_input(&mut app, key_event(KeyCode::F(3))));
    assert!(app.get_viewer().is_none());

    // Test closing with Enter
    app.set_overlay(Overlay::Viewer(Viewer {
        title: "test.txt".to_string(),
        lines: vec!["Line 1".to_string()],
        scroll: ScrollTop::TOP,
    }));
    assert!(process_popup_input(&mut app, key_event(KeyCode::Enter)));
    assert!(app.get_viewer().is_none());

    Ok(())
}

#[test]
fn test_process_popup_input_viewer_max_scroll() -> io::Result<()> {
    let mut app = App::new().expect("test setup");

    // Create a viewer with limited lines
    app.set_overlay(Overlay::Viewer(Viewer {
        title: "test.txt".to_string(),
        lines: vec!["Line 1".to_string(), "Line 2".to_string()],
        scroll: ScrollTop::TOP,
    }));

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
    app.set_overlay(Overlay::Viewer(Viewer {
        title: "test.txt".to_string(),
        lines: vec!["only line".to_string()],
        scroll: ScrollTop::TOP,
    }));
    process_popup_input(&mut app, key_event(KeyCode::Down));
    assert_eq!(app.get_viewer().unwrap().scroll, ScrollTop::TOP);

    // Empty lines: scroll should stay at 0
    app.set_overlay(Overlay::Viewer(Viewer {
        title: "test.txt".to_string(),
        lines: vec![],
        scroll: ScrollTop::TOP,
    }));
    process_popup_input(&mut app, key_event(KeyCode::Down));
    assert_eq!(app.get_viewer().unwrap().scroll, ScrollTop::TOP);

    // 3 lines: max scroll should be exactly 2
    app.set_overlay(Overlay::Viewer(Viewer {
        title: "test.txt".to_string(),
        lines: vec!["a".into(), "b".into(), "c".into()],
        scroll: ScrollTop::TOP,
    }));
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
    assert_eq!(viewer.title, "test.txt");
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

    // Non-UTF-8 *text* is decoded lossily rather than failing the whole action.
    handle_view(&mut app).expect("lossy decode must not fail");
    let viewer = app.get_viewer().expect("viewer must open");
    assert_eq!(viewer.lines, vec!["\u{fffd}".to_owned()]);

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
    assert_eq!(app.status_text(), "Cannot view a directory");

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
