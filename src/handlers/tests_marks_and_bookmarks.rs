//! Unit tests: marks and bookmarks (split out to keep files under 500 lines).
#![allow(unused_imports)]

use super::tests::*;
use crate::models::FileName;
use crate::scroll::ScrollTop;

use super::file_ops::{
    format_operation_status, get_source_paths, rename_or_fail, FORCE_RENAME_FAILURE,
    RENAME_TEST_MUTEX,
};

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

#[test]
fn test_execute_command_mark_all() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    fs::write(temp_dir.path().join("a.txt"), b"a").expect("test setup");
    fs::write(temp_dir.path().join("b.txt"), b"b").expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    execute_command(&mut app, "mark all").expect("test setup");
    assert!(app.left_panel.marked.contains("a.txt"));
    assert!(app.left_panel.marked.contains("b.txt"));
    // ".." should not be marked
    assert!(!app.left_panel.marked.contains(".."));
    assert!(app.status_text().contains("Marked"));

    Ok(())
}

// -----------------------------------------------------------------------
// handle_delete_confirm -- edge cases
// -----------------------------------------------------------------------

/// No selection (state = None) returns Ok without setting `delete_confirm`.
#[test]
fn test_handle_delete_confirm_no_selection() -> io::Result<()> {
    let mut app = App::new().expect("test setup");
    app.left_panel.select(None);

    handle_delete_confirm(&mut app).expect("should return Ok");

    assert!(!matches!(app.overlay, Overlay::DeleteConfirm(_)));

    Ok(())
}

/// Selecting ".." does NOT set `delete_confirm` (parent directory is protected).
#[test]
fn test_handle_delete_confirm_skips_parent_entry() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    // ".." is always the first entry when a parent exists.
    let dotdot_index = app
        .left_panel
        .entries
        .iter()
        .position(|e| e.name == "..")
        .expect(".. entry must exist");

    app.left_panel.select(Some(dotdot_index));

    handle_delete_confirm(&mut app).expect("should return Ok");

    // delete_confirm must NOT be set -- we must not delete "..".
    assert!(!matches!(app.overlay, Overlay::DeleteConfirm(_)));

    Ok(())
}

/// Marked files take precedence over the current selection in `delete_confirm`.
#[test]
fn test_handle_delete_confirm_marked_takes_precedence() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let marked_file = temp_dir.path().join("marked.txt");
    let selected_file = temp_dir.path().join("selected.txt");
    File::create(&marked_file).expect("test setup");
    File::create(&selected_file).expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    // Select `selected_file` in the panel.
    let selected_index = app
        .left_panel
        .entries
        .iter()
        .position(|e| e.name == "selected.txt")
        .expect("selected.txt must be in entries");
    app.left_panel.select(Some(selected_index));

    // Mark `marked_file` but NOT `selected_file`.
    app.left_panel.marked.insert(FileName::from("marked.txt"));

    handle_delete_confirm(&mut app).expect("should return Ok");

    if let Overlay::DeleteConfirm(ref paths) = app.overlay {
        // The confirm list must contain the marked file, not the selected one.
        assert!(paths.iter().any(|p| p.ends_with("marked.txt")));
        assert!(!paths.iter().any(|p| p.ends_with("selected.txt")));
    } else {
        panic!("Expected DeleteConfirm overlay");
    }

    Ok(())
}

// -----------------------------------------------------------------------
// Bookmarks overlay -- F2 key closes overlay
// -----------------------------------------------------------------------

// -----------------------------------------------------------------------
// Search overlay -- backspace on single character leaves Some("")
// -----------------------------------------------------------------------

// -----------------------------------------------------------------------
// execute_command -- direct unit tests for every command arm
// -----------------------------------------------------------------------

#[test]
fn test_execute_command_pwd_direct() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    execute_command(&mut app, "pwd").expect("pwd command");

    assert!(
        app.status_text()
            .contains(&temp_dir.path().display().to_string()),
        "status_message should contain the current panel path"
    );

    Ok(())
}

#[test]
fn test_execute_command_unmark() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    fs::write(temp_dir.path().join("a.txt"), b"a").expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.left_panel.marked.insert(FileName::from("a.txt"));

    execute_command(&mut app, "unmark").expect("test setup");
    assert!(app.left_panel.marked.is_empty());
    assert!(app.status_text().contains("Unmarked"));

    Ok(())
}

#[test]
fn test_execute_command_sort_size_direct() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    execute_command(&mut app, "sort size").expect("sort size");

    assert_eq!(app.get_active_panel().sort_mode, SortMode::Size);

    Ok(())
}

#[test]
fn test_execute_command_sort_name_direct() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.get_active_panel().sort_mode = SortMode::Size;

    execute_command(&mut app, "sort name").expect("sort name");

    assert_eq!(app.get_active_panel().sort_mode, SortMode::Name);

    Ok(())
}

#[test]
fn test_execute_command_bookmark_set_slot_9() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    execute_command(&mut app, "bookmark set 9").expect("bookmark set 9");

    assert!(
        app.bookmarks
            .get(BookmarkSlot::from_usize(9).expect("slot"))
            .is_some(),
        "bookmark slot 9 should be set"
    );
    assert_eq!(
        app.bookmarks
            .get(BookmarkSlot::from_usize(9).expect("slot"))
            .expect("set"),
        temp_dir.path()
    );

    Ok(())
}

#[test]
fn test_execute_command_mark_all_direct() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    File::create(temp_dir.path().join("file1.txt")).expect("test setup");
    File::create(temp_dir.path().join("file2.txt")).expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    execute_command(&mut app, "mark all").expect("mark all");

    let marked = &app.get_active_panel().marked;
    assert!(!marked.is_empty(), "mark all should mark entries");
    assert!(marked.contains("file1.txt"));
    assert!(marked.contains("file2.txt"));
    // ".." should never be marked
    assert!(!marked.contains(".."));

    Ok(())
}

#[test]
fn test_execute_command_unmark_direct() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    File::create(temp_dir.path().join("file.txt")).expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.left_panel.marked.insert(FileName::from("file.txt"));
    assert!(!app.left_panel.marked.is_empty());

    execute_command(&mut app, "unmark").expect("unmark");

    assert!(
        app.get_active_panel().marked.is_empty(),
        "unmark should clear all marks"
    );
    assert!(app.status_text().contains("Unmarked"));

    Ok(())
}

#[test]
fn test_execute_command_help() -> io::Result<()> {
    let mut app = App::new().expect("test setup");

    execute_command(&mut app, "help").expect("test setup");
    assert!(matches!(app.overlay, Overlay::Help));

    Ok(())
}

#[test]
fn test_execute_command_help_direct() -> io::Result<()> {
    let mut app = App::new().expect("test setup");
    assert!(!matches!(app.overlay, Overlay::Help));

    execute_command(&mut app, "help").expect("help command");

    assert!(
        matches!(app.overlay, Overlay::Help),
        "help command should set overlay to Help"
    );

    Ok(())
}

#[test]
fn test_execute_command_exit() -> io::Result<()> {
    let mut app = App::new().expect("test setup");

    execute_command(&mut app, "exit").expect("test setup");
    assert!(!app.lifecycle.is_running());

    Ok(())
}

#[test]
fn test_execute_command_quit_direct() -> io::Result<()> {
    let mut app = App::new().expect("test setup");
    assert!(app.lifecycle.is_running());

    execute_command(&mut app, "quit").expect("quit command");

    assert!(
        !app.lifecycle.is_running(),
        "quit command should set should_quit = true"
    );

    Ok(())
}

#[test]
#[cfg(not(windows))]
fn test_execute_command_windows_system_aliases() -> io::Result<()> {
    for alias in ["winmsg", "winmsgs", "winerr", "winerrors"] {
        let mut app = App::new().expect("test setup");

        execute_command(&mut app, alias)?;

        assert_eq!(
            app.status_text(),
            "Windows system checker is only available on Windows"
        );
        assert!(app.get_viewer().is_none());
    }

    Ok(())
}

#[test]
#[cfg(not(windows))]
fn test_execute_command_windows_performance_aliases() -> io::Result<()> {
    for alias in ["winperf", "wincheck"] {
        let mut app = App::new().expect("test setup");

        execute_command(&mut app, alias)?;

        assert_eq!(
            app.status_text(),
            "Windows performance checker is only available on Windows"
        );
        assert!(app.get_viewer().is_none());
    }

    Ok(())
}

#[test]
fn test_execute_command_cd_relative() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let subdir = temp_dir.path().join("child");
    fs::create_dir(&subdir).expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    execute_command(&mut app, "cd child").expect("test setup");
    assert!(app.status_text().contains("Opened"));

    Ok(())
}

#[test]
fn test_execute_command_unknown_returns_error() -> io::Result<()> {
    let mut app = App::new().expect("test setup");

    // Unknown commands are dispatched to the shell; the shell process
    // runs successfully (Ok) even though the command is not found.
    let result = execute_command(&mut app, "frobnicator");
    assert!(result.is_ok());
    // The status_message should capture the shell's error output.
    assert!(!app.status_text().is_empty());

    Ok(())
}

#[test]
fn test_execute_command_bookmark_go_0_set_then_navigate() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let bookmark_dir = temp_dir.path().join("bookmark_target");
    fs::create_dir(&bookmark_dir).expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.bookmarks.set(
        BookmarkSlot::from_usize(0).expect("slot"),
        bookmark_dir.clone(),
    );

    execute_command(&mut app, "bookmark go 0").expect("bookmark go 0");

    assert_eq!(app.get_active_panel().path, bookmark_dir);
    assert!(app.status_text().contains("Opened bookmark 0"));

    Ok(())
}

// --- Viewer with single line ---

#[test]
fn test_process_popup_input_viewer_single_line_no_scroll() -> io::Result<()> {
    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::Viewer(Viewer {
        title: "one.txt".to_string(),
        lines: vec!["only line".to_string()],
        scroll: ScrollTop::TOP,
    }));

    // Try to scroll down - should stay at 0
    assert!(process_popup_input(&mut app, key_event(KeyCode::Down)));
    assert_eq!(app.get_viewer().unwrap().scroll, ScrollTop::TOP);

    Ok(())
}
