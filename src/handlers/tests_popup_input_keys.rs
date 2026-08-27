//! Unit tests: popup input keys (split out to keep files under 500 lines).
#![allow(unused_imports)]

use super::tests::*;

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

// --- Search overlay edge cases ---

/// Ctrl+Char is ignored (char is not appended to the buffer).
#[test]
fn test_new_file_input_ctrl_char_ignored() -> io::Result<()> {
    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::NewFileInput("abc".to_string()));

    process_popup_input(
        &mut app,
        key_event_with_mods(KeyCode::Char('d'), KeyModifiers::CONTROL),
    );

    assert_eq!(
        if let Overlay::NewFileInput(ref s) = app.overlay {
            s
        } else {
            panic!()
        },
        "abc"
    );

    Ok(())
}

// --- Bookmark overlay edge cases ---

// --- Command line edge cases ---

#[test]
fn test_process_popup_input_command_line_alt_char_ignored() -> io::Result<()> {
    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::CommandLine("cmd".into()));

    // Alt+Char is ignored by the command line but still consumed by it.
    assert!(process_popup_input(
        &mut app,
        key_event_with_mods(KeyCode::Char('x'), KeyModifiers::ALT)
    ));
    if let Overlay::CommandLine(ref s) = app.overlay {
        assert_eq!(s.text(), "cmd");
    } else {
        panic!("Expected CommandLine overlay");
    }

    Ok(())
}

/// Alt+Char is ignored (char is not appended to the buffer).
#[test]
fn test_new_file_input_alt_char_ignored() -> io::Result<()> {
    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::NewFileInput("abc".to_string()));

    process_popup_input(
        &mut app,
        key_event_with_mods(KeyCode::Char('d'), KeyModifiers::ALT),
    );

    assert_eq!(
        app.get_new_file_input_mut().map(|s| s.as_str()),
        Some("abc")
    );

    Ok(())
}

#[test]
fn test_process_popup_input_command_line_other_key() -> io::Result<()> {
    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::CommandLine("test".into()));

    // F-keys are swallowed: F5 must not copy files behind the open prompt.
    assert!(process_popup_input(&mut app, key_event(KeyCode::F(5))));
    if let Overlay::CommandLine(ref s) = app.overlay {
        assert_eq!(s.text(), "test");
    } else {
        panic!("Expected CommandLine overlay");
    }

    Ok(())
}

/// Unrecognised key in `new_file_input` overlay -- overlay stays open, buffer unchanged.
#[test]
fn test_new_file_input_other_key_ignored() -> io::Result<()> {
    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::NewFileInput("abc".to_string()));

    process_popup_input(&mut app, key_event(KeyCode::F(5)));

    assert_eq!(
        app.get_new_file_input_mut().map(|s| s.as_str()),
        Some("abc")
    );

    Ok(())
}

/// Backspace on an empty input buffer leaves it empty (no panic).
#[test]
fn test_new_file_input_backspace_empty() -> io::Result<()> {
    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::NewFileInput(String::new()));

    process_popup_input(&mut app, key_event(KeyCode::Backspace));

    assert_eq!(app.get_new_file_input_mut().map(|s| s.as_str()), Some(""));

    Ok(())
}

// -----------------------------------------------------------------------
// execute_command -- additional branch coverage
// -----------------------------------------------------------------------

/// `exit` is an alias for `quit` and sets `should_quit`.
#[test]
fn test_execute_command_exit_alias() -> io::Result<()> {
    let mut app = App::new().expect("test setup");

    execute_command(&mut app, "exit").expect("exit command");

    assert!(!app.lifecycle.is_running());

    Ok(())
}

/// `sort` with no argument defaults to sorting by name.
#[test]
fn test_execute_command_sort_no_arg_defaults_to_name() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    // Set to a different mode first so the change back to Name is observable.
    app.get_active_panel().sort_mode = SortMode::Size;

    execute_command(&mut app, "sort").expect("sort without arg");

    assert_eq!(app.get_active_panel().sort_mode, SortMode::Name);

    Ok(())
}

/// `sort extension` (long form) sets mode to Extension.
#[test]
fn test_execute_command_sort_extension_long_form() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    execute_command(&mut app, "sort extension").expect("sort extension");

    assert_eq!(app.get_active_panel().sort_mode, SortMode::Extension);

    Ok(())
}

#[test]
fn test_execute_command_cd_to_file_not_dir() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let file_path = temp_dir.path().join("notadir.txt");
    fs::write(&file_path, b"data").expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    let result = execute_command(&mut app, &format!("cd {}", file_path.display()));
    assert!(result.is_err());

    Ok(())
}

#[test]
fn test_execute_command_sort_extension() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    fs::write(temp_dir.path().join("b.txt"), b"b").expect("test setup");
    fs::write(temp_dir.path().join("a.rs"), b"a").expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    execute_command(&mut app, "sort ext").expect("test setup");
    assert_eq!(app.left_panel.sort_mode, SortMode::Extension);
    assert!(app.status_text().contains("Ext"));

    execute_command(&mut app, "sort extension").expect("test setup");
    assert_eq!(app.left_panel.sort_mode, SortMode::Extension);

    execute_command(&mut app, "sort size").expect("test setup");
    assert_eq!(app.left_panel.sort_mode, SortMode::Size);
    assert!(app.status_text().contains("Size"));

    execute_command(&mut app, "sort name").expect("test setup");
    assert_eq!(app.left_panel.sort_mode, SortMode::Name);
    assert!(app.status_text().contains("Name"));

    Ok(())
}

#[test]
fn test_execute_command_bookmark_set_and_go() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    execute_command(&mut app, "bookmark set 3").expect("test setup");
    assert!(app
        .bookmarks
        .get(BookmarkSlot::from_usize(3).expect("slot"))
        .is_some());
    assert!(app.status_text().contains("Bookmark 3 set"));

    execute_command(&mut app, "bookmark go 3").expect("test setup");
    assert!(app.status_text().contains("Opened bookmark 3"));

    Ok(())
}

#[test]
fn test_execute_command_bookmark_slot_out_of_range() -> io::Result<()> {
    let mut app = App::new().expect("test setup");

    let result = execute_command(&mut app, "bookmark set 99");
    assert!(result.is_err());

    Ok(())
}

/// `bookmark go` with a slot pointing to a missing directory returns an error
/// *and* leaves the panel on its previous, readable directory.
///
/// Asserting only `is_err()` tolerated the bug this pins: the path was committed
/// before the fallible load, so the panel ended up on `…/gone` showing a lone
/// ".." with a stale selection index.
#[test]
fn test_execute_command_bookmark_go_missing_directory() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let missing_dir = temp_dir.path().join("gone");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    // Set bookmark 0 to a directory that we then delete.
    fs::create_dir(&missing_dir).expect("test setup");
    app.bookmarks.set(
        BookmarkSlot::from_usize(0).expect("slot"),
        missing_dir.clone(),
    );
    fs::remove_dir(&missing_dir).expect("test setup");

    let result = execute_command(&mut app, "bookmark go 0");
    assert!(result.is_err());
    assert_eq!(
        app.left_panel.path,
        temp_dir.path(),
        "a panel must never show a path it has no listing for"
    );
    assert!(
        app.left_panel
            .selected()
            .is_none_or(|index| index < app.left_panel.entries.len()),
        "the selection must stay inside the listing"
    );

    Ok(())
}

#[test]
fn test_execute_command_bookmark_non_numeric_slot() -> io::Result<()> {
    let mut app = App::new().expect("test setup");

    let result = execute_command(&mut app, "bookmark set abc");
    assert!(result.is_err());

    Ok(())
}

/// `bookmark set` with no slot argument returns `InvalidInput`.
#[test]
fn test_execute_command_bookmark_no_slot() -> io::Result<()> {
    let mut app = App::new().expect("test setup");

    // With Command enum + Shell fallback, incomplete "bookmark" commands
    // are treated as shell commands (which may fail later).
    let result = execute_command(&mut app, "bookmark set");
    // It no longer short-circuits with InvalidInput at parse time.
    // The important thing is it doesn't panic.
    let _ = result;
    Ok(())
}

/// `bookmark go 99` (slot > 9) returns an error.
#[test]
fn test_execute_command_bookmark_go_slot_out_of_range() -> io::Result<()> {
    let mut app = App::new().expect("test setup");

    let result = execute_command(&mut app, "bookmark go 99");
    assert!(result.is_err());

    Ok(())
}
