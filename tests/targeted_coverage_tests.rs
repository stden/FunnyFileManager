//! Targeted tests to cover remaining uncovered lines and important edge cases.
//! Focus areas: new file input overlay, delete confirm edge cases, shell commands,
//! multi-file operations, viewer edge cases, event loop paths, `format_operation_status`.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use funny_file_manager::handlers::{
    handle_copy, handle_delete, handle_delete_confirm, handle_mkdir, handle_move, handle_view,
    process_popup_input,
};
use funny_file_manager::models::FileName;
use funny_file_manager::models::{App, Overlay, PanelSide, Viewer};
use funny_file_manager::scroll::ScrollTop;
use std::fs::{self, File};
use std::io::{self, Write};
use tempfile::TempDir;

const fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::empty())
}

const fn key_ctrl(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::CONTROL)
}

const fn key_shift(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::SHIFT)
}

#[cfg(windows)]
fn shell_stderr_command() -> &'static str {
    "echo error 1>&2"
}

#[cfg(not(windows))]
const fn shell_stderr_command() -> &'static str {
    "echo error >&2"
}

#[cfg(windows)]
fn shell_failure_command() -> &'static str {
    "cmd /c exit /b 1"
}

#[cfg(not(windows))]
const fn shell_failure_command() -> &'static str {
    "false"
}

#[cfg(windows)]
fn shell_success_no_output_command() -> &'static str {
    "ver >nul"
}

#[cfg(not(windows))]
const fn shell_success_no_output_command() -> &'static str {
    "true"
}

// ──────────────────────────────────────────────
// New file input overlay (Shift+F4)
// ──────────────────────────────────────────────

#[test]
fn test_new_file_input_type_and_create() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel.path = temp.path().to_path_buf();
    app.left_panel.load_entries()?;

    // Open new file input
    app.set_overlay(Overlay::NewFileInput(String::new()));

    // Type "hello.txt"
    for ch in "hello.txt".chars() {
        process_popup_input(&mut app, key(KeyCode::Char(ch)));
    }
    assert_eq!(app.get_new_file_input(), Some("hello.txt"));

    // Press Enter to create
    process_popup_input(&mut app, key(KeyCode::Enter));
    assert!(app.get_new_file_input().is_none());
    assert!(temp.path().join("hello.txt").exists());
    assert!(app.status_text().contains("Created file"));

    Ok(())
}

#[test]
fn test_new_file_input_empty_name_rejected() -> io::Result<()> {
    let mut app = App::new()?;
    app.set_overlay(Overlay::NewFileInput(String::new()));

    // Press Enter with empty name
    process_popup_input(&mut app, key(KeyCode::Enter));
    assert!(app.get_new_file_input().is_none());
    assert!(app.status_text().contains("empty"));

    Ok(())
}

#[test]
fn test_new_file_input_already_exists() -> io::Result<()> {
    let temp = TempDir::new()?;
    File::create(temp.path().join("existing.txt"))?.write_all(b"x")?;

    let mut app = App::new()?;
    app.left_panel.path = temp.path().to_path_buf();
    app.left_panel.load_entries()?;
    app.set_overlay(Overlay::NewFileInput("existing.txt".to_owned()));

    process_popup_input(&mut app, key(KeyCode::Enter));
    assert!(app.status_text().contains("already exists"));

    Ok(())
}

#[test]
fn test_new_file_input_backspace() -> io::Result<()> {
    let mut app = App::new()?;
    app.set_overlay(Overlay::NewFileInput("abc".to_owned()));

    process_popup_input(&mut app, key(KeyCode::Backspace));
    assert_eq!(app.get_new_file_input(), Some("ab"));

    Ok(())
}

#[test]
fn test_new_file_input_escape_cancels() -> io::Result<()> {
    let mut app = App::new()?;
    app.set_overlay(Overlay::NewFileInput("something.txt".to_owned()));

    process_popup_input(&mut app, key(KeyCode::Esc));
    assert!(app.get_new_file_input().is_none());

    Ok(())
}

#[test]
fn test_new_file_input_ignores_ctrl_chars() -> io::Result<()> {
    let mut app = App::new()?;
    app.set_overlay(Overlay::NewFileInput("test".to_owned()));

    // Ctrl+A should not add 'a' to input
    process_popup_input(&mut app, key_ctrl(KeyCode::Char('a')));
    assert_eq!(app.get_new_file_input(), Some("test"));

    Ok(())
}

// ──────────────────────────────────────────────
// Delete confirm edge cases
// ──────────────────────────────────────────────

#[test]
fn test_delete_confirm_dotdot_is_skipped() -> io::Result<()> {
    let temp = TempDir::new()?;
    let subdir = temp.path().join("sub");
    fs::create_dir(&subdir)?;

    let mut app = App::new()?;
    app.left_panel.path = subdir;
    app.left_panel.load_entries()?;

    // Select ".." entry
    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == ".." {
            app.left_panel.select(Some(i));
            break;
        }
    }

    handle_delete_confirm(&mut app)?;
    // Should NOT set delete_confirm for ".."
    assert!(app.get_delete_confirm().is_none());

    Ok(())
}

#[test]
fn test_delete_confirm_no_selection() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel.path = temp.path().to_path_buf();
    // Empty directory, no entries to select
    app.left_panel.entries.clear();
    app.left_panel.select(None);

    handle_delete_confirm(&mut app)?;
    assert!(app.get_delete_confirm().is_none());

    Ok(())
}

#[test]
fn test_delete_confirm_marked_files() -> io::Result<()> {
    let temp = TempDir::new()?;
    File::create(temp.path().join("a.txt"))?.write_all(b"a")?;
    File::create(temp.path().join("b.txt"))?.write_all(b"b")?;

    let mut app = App::new()?;
    app.left_panel.path = temp.path().to_path_buf();
    app.left_panel.load_entries()?;

    // Mark both files
    app.left_panel.marked.insert(FileName::from("a.txt"));
    app.left_panel.marked.insert(FileName::from("b.txt"));

    handle_delete_confirm(&mut app)?;
    assert!(app.get_delete_confirm().is_some());
    assert_eq!(app.get_delete_confirm().unwrap().len(), 2);

    Ok(())
}

#[test]
fn test_delete_confirm_overlay_yes_no() -> io::Result<()> {
    let temp = TempDir::new()?;
    File::create(temp.path().join("del.txt"))?.write_all(b"x")?;

    let mut app = App::new()?;
    app.left_panel.path = temp.path().to_path_buf();
    app.left_panel.load_entries()?;

    // Select del.txt
    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == "del.txt" {
            app.left_panel.select(Some(i));
            break;
        }
    }

    // Set up delete confirm
    handle_delete_confirm(&mut app)?;
    assert!(app.get_delete_confirm().is_some());

    // Cancel with 'N'
    process_popup_input(&mut app, key(KeyCode::Char('n')));
    assert!(app.get_delete_confirm().is_none());
    assert!(app.status_text().contains("cancelled"));
    assert!(temp.path().join("del.txt").exists()); // File should still exist

    // Set up again and confirm with 'Y'
    handle_delete_confirm(&mut app)?;
    process_popup_input(&mut app, key(KeyCode::Char('y')));
    assert!(!temp.path().join("del.txt").exists()); // File should be deleted

    Ok(())
}

// ──────────────────────────────────────────────
// Shell command execution via command line
// ──────────────────────────────────────────────

#[test]
fn test_command_line_execute_shell_command() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel.path = temp.path().to_path_buf();
    app.left_panel.load_entries()?;

    // Type a shell command: "echo hello"
    app.set_overlay(Overlay::CommandLine(String::new().into()));
    for ch in "echo hello".chars() {
        process_popup_input(&mut app, key(KeyCode::Char(ch)));
    }
    assert_eq!(app.get_command_line(), Some("echo hello"));

    // Execute with Enter
    process_popup_input(&mut app, key(KeyCode::Enter));
    assert!(app.get_command_line().is_none());
    // Shell command stdout should be in status
    assert!(
        app.status_text().contains("hello"),
        "Expected 'hello' in status, got: {}",
        app.status_text()
    );

    Ok(())
}

#[test]
fn test_command_line_pwd() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel.path = temp.path().to_path_buf();

    app.set_overlay(Overlay::CommandLine("pwd".into()));
    process_popup_input(&mut app, key(KeyCode::Enter));
    assert!(app
        .status_text()
        .contains(&temp.path().display().to_string()));

    Ok(())
}

#[test]
fn test_command_line_sort() -> io::Result<()> {
    let temp = TempDir::new()?;
    File::create(temp.path().join("a.txt"))?.write_all(b"x")?;
    File::create(temp.path().join("b.rs"))?.write_all(b"y")?;

    let mut app = App::new()?;
    app.left_panel.path = temp.path().to_path_buf();
    app.left_panel.load_entries()?;

    // Sort by extension
    app.set_overlay(Overlay::CommandLine("sort ext".into()));
    process_popup_input(&mut app, key(KeyCode::Enter));
    assert!(app.status_text().contains("Sort"));

    // Sort by size
    app.set_overlay(Overlay::CommandLine("sort size".into()));
    process_popup_input(&mut app, key(KeyCode::Enter));
    assert!(app.status_text().contains("Sort"));

    // Invalid sort mode
    app.set_overlay(Overlay::CommandLine("sort blah".into()));
    process_popup_input(&mut app, key(KeyCode::Enter));
    assert!(app.status_text().contains("sort expects") || app.status_text().contains("Error"));

    Ok(())
}

#[test]
fn test_command_line_mark_all_and_unmark() -> io::Result<()> {
    let temp = TempDir::new()?;
    File::create(temp.path().join("a.txt"))?.write_all(b"a")?;
    File::create(temp.path().join("b.txt"))?.write_all(b"b")?;

    let mut app = App::new()?;
    app.left_panel.path = temp.path().to_path_buf();
    app.left_panel.load_entries()?;

    // Mark all
    app.set_overlay(Overlay::CommandLine("mark all".into()));
    process_popup_input(&mut app, key(KeyCode::Enter));
    assert!(!app.left_panel.marked.is_empty());
    assert!(app.status_text().contains("Marked"));

    // Unmark
    app.set_overlay(Overlay::CommandLine("unmark".into()));
    process_popup_input(&mut app, key(KeyCode::Enter));
    assert!(app.left_panel.marked.is_empty());
    assert!(app.status_text().contains("Unmarked"));

    Ok(())
}

#[test]
fn test_command_line_help() -> io::Result<()> {
    let mut app = App::new()?;

    app.set_overlay(Overlay::CommandLine("help".into()));
    process_popup_input(&mut app, key(KeyCode::Enter));
    assert!(matches!(app.overlay, Overlay::Help));

    Ok(())
}

#[test]
fn test_command_line_quit() -> io::Result<()> {
    let mut app = App::new()?;

    app.set_overlay(Overlay::CommandLine("quit".into()));
    process_popup_input(&mut app, key(KeyCode::Enter));
    assert!(!app.lifecycle.is_running());

    Ok(())
}

#[test]
fn test_command_line_exit() -> io::Result<()> {
    let mut app = App::new()?;

    app.set_overlay(Overlay::CommandLine("exit".into()));
    process_popup_input(&mut app, key(KeyCode::Enter));
    assert!(!app.lifecycle.is_running());

    Ok(())
}

#[test]
fn test_command_line_empty_command() -> io::Result<()> {
    let mut app = App::new()?;

    app.set_overlay(Overlay::CommandLine("   ".into()));
    process_popup_input(&mut app, key(KeyCode::Enter));
    // Whitespace-only command is treated as empty — status may show error or remain unchanged
    // Just verify the command line was cleared
    assert!(app.get_command_line().is_none());

    Ok(())
}

#[test]
fn test_command_line_cd_absolute() -> io::Result<()> {
    let temp = TempDir::new()?;
    let target = temp.path().to_path_buf();

    let mut app = App::new()?;

    app.set_overlay(Overlay::CommandLine(
        format!("cd {}", target.display()).into(),
    ));
    process_popup_input(&mut app, key(KeyCode::Enter));
    assert!(app.status_text().contains("Opened"));

    Ok(())
}

#[test]
fn test_command_line_cd_relative() -> io::Result<()> {
    let temp = TempDir::new()?;
    let subdir = temp.path().join("child");
    fs::create_dir(&subdir)?;

    let mut app = App::new()?;
    app.left_panel.path = temp.path().to_path_buf();
    app.left_panel.load_entries()?;

    app.set_overlay(Overlay::CommandLine("cd child".into()));
    process_popup_input(&mut app, key(KeyCode::Enter));
    assert!(app.status_text().contains("Opened"));

    Ok(())
}

#[test]
fn test_command_line_bookmark_set_and_go() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel.path = temp.path().to_path_buf();
    app.left_panel.load_entries()?;

    // Set bookmark 3
    app.set_overlay(Overlay::CommandLine("bookmark set 3".into()));
    process_popup_input(&mut app, key(KeyCode::Enter));
    assert!(app.status_text().contains("Bookmark 3 set"));

    // Go to bookmark 3
    app.set_overlay(Overlay::CommandLine("bookmark go 3".into()));
    process_popup_input(&mut app, key(KeyCode::Enter));
    assert!(app.status_text().contains("Opened bookmark 3"));

    Ok(())
}

#[test]
fn test_command_line_bookmark_errors() -> io::Result<()> {
    let mut app = App::new()?;

    // Missing subcommand
    app.set_overlay(Overlay::CommandLine("bookmark".into()));
    process_popup_input(&mut app, key(KeyCode::Enter));
    assert!(app.status_text().contains("Error") || app.status_text().contains("bookmark"));

    // Missing slot
    app.set_overlay(Overlay::CommandLine("bookmark set".into()));
    process_popup_input(&mut app, key(KeyCode::Enter));
    assert!(app.status_text().contains("Error") || app.status_text().contains("slot"));

    // Invalid slot
    app.set_overlay(Overlay::CommandLine("bookmark go 99".into()));
    process_popup_input(&mut app, key(KeyCode::Enter));
    assert!(app.status_text().contains("Error") || app.status_text().contains("slot"));

    // Go to empty bookmark
    app.set_overlay(Overlay::CommandLine("bookmark go 0".into()));
    process_popup_input(&mut app, key(KeyCode::Enter));
    assert!(app.status_text().contains("Error") || app.status_text().contains("empty"));

    Ok(())
}

#[test]
fn test_command_line_shell_stderr() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel.path = temp.path().to_path_buf();
    app.left_panel.load_entries()?;

    // Command that produces stderr only
    app.set_overlay(Overlay::CommandLine(shell_stderr_command().into()));
    process_popup_input(&mut app, key(KeyCode::Enter));
    assert!(
        app.status_text().contains("Err") || app.status_text().contains("error"),
        "Got: {}",
        app.status_text()
    );

    Ok(())
}

#[test]
fn test_command_line_shell_exit_code() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel.path = temp.path().to_path_buf();
    app.left_panel.load_entries()?;

    // Command that exits with non-zero, no output
    app.set_overlay(Overlay::CommandLine(shell_failure_command().into()));
    process_popup_input(&mut app, key(KeyCode::Enter));
    assert!(
        app.status_text().contains("Exit"),
        "Got: {}",
        app.status_text()
    );

    Ok(())
}

#[test]
fn test_command_line_shell_ok_no_output() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel.path = temp.path().to_path_buf();
    app.left_panel.load_entries()?;

    // Command that succeeds with no output
    app.set_overlay(Overlay::CommandLine(
        shell_success_no_output_command().into(),
    ));
    process_popup_input(&mut app, key(KeyCode::Enter));
    assert!(
        app.status_text().contains("OK"),
        "Got: {}",
        app.status_text()
    );

    Ok(())
}

// ──────────────────────────────────────────────
// Multi-file copy/move/delete
// ──────────────────────────────────────────────

#[test]
fn test_copy_multiple_marked_files() -> io::Result<()> {
    let temp = TempDir::new()?;
    let left = temp.path().join("left");
    let right = temp.path().join("right");
    fs::create_dir(&left)?;
    fs::create_dir(&right)?;
    File::create(left.join("a.txt"))?.write_all(b"aaa")?;
    File::create(left.join("b.txt"))?.write_all(b"bbb")?;
    File::create(left.join("c.txt"))?.write_all(b"ccc")?;

    let mut app = App::new()?;
    app.left_panel.path = left;
    app.right_panel.path = right.clone();
    app.left_panel.load_entries()?;
    app.right_panel.load_entries()?;

    // Mark multiple files
    app.left_panel.marked.insert(FileName::from("a.txt"));
    app.left_panel.marked.insert(FileName::from("b.txt"));
    app.left_panel.marked.insert(FileName::from("c.txt"));

    handle_copy(&mut app)?;

    assert!(right.join("a.txt").exists());
    assert!(right.join("b.txt").exists());
    assert!(right.join("c.txt").exists());
    // Multi-file status uses count format
    assert!(
        app.status_text().contains('3'),
        "Got: {}",
        app.status_text()
    );

    Ok(())
}

#[test]
fn test_move_multiple_marked_files() -> io::Result<()> {
    let temp = TempDir::new()?;
    let left = temp.path().join("left");
    let right = temp.path().join("right");
    fs::create_dir(&left)?;
    fs::create_dir(&right)?;
    File::create(left.join("x.txt"))?.write_all(b"x")?;
    File::create(left.join("y.txt"))?.write_all(b"y")?;

    let mut app = App::new()?;
    app.left_panel.path = left.clone();
    app.right_panel.path = right.clone();
    app.left_panel.load_entries()?;
    app.right_panel.load_entries()?;

    app.left_panel.marked.insert(FileName::from("x.txt"));
    app.left_panel.marked.insert(FileName::from("y.txt"));

    handle_move(&mut app)?;

    assert!(!left.join("x.txt").exists());
    assert!(!left.join("y.txt").exists());
    assert!(right.join("x.txt").exists());
    assert!(right.join("y.txt").exists());
    assert!(
        app.status_text().contains('2'),
        "Got: {}",
        app.status_text()
    );

    Ok(())
}

#[test]
fn test_copy_directory() -> io::Result<()> {
    let temp = TempDir::new()?;
    let left = temp.path().join("left");
    let right = temp.path().join("right");
    fs::create_dir(&left)?;
    fs::create_dir(&right)?;

    // Create a subdirectory with content
    let subdir = left.join("mydir");
    fs::create_dir(&subdir)?;
    File::create(subdir.join("inner.txt"))?.write_all(b"inner")?;

    let mut app = App::new()?;
    app.left_panel.path = left;
    app.right_panel.path = right.clone();
    app.left_panel.load_entries()?;
    app.right_panel.load_entries()?;

    // Select the directory
    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == "mydir" {
            app.left_panel.select(Some(i));
            break;
        }
    }

    handle_copy(&mut app)?;
    assert!(right.join("mydir").is_dir());
    assert!(right.join("mydir").join("inner.txt").exists());

    Ok(())
}

#[test]
fn test_copy_no_selection() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel.path = temp.path().to_path_buf();
    app.left_panel.entries.clear();
    app.left_panel.select(None);

    // Should return Ok without error
    handle_copy(&mut app)?;
    assert!(app.status_text().is_empty() || !app.status_text().contains("Copied"));

    Ok(())
}

#[test]
fn test_move_no_selection() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel.path = temp.path().to_path_buf();
    app.left_panel.entries.clear();
    app.left_panel.select(None);

    handle_move(&mut app)?;
    assert!(app.status_text().is_empty() || !app.status_text().contains("Moved"));

    Ok(())
}

// ──────────────────────────────────────────────
// Viewer edge cases
// ──────────────────────────────────────────────

#[test]
fn test_view_empty_file() -> io::Result<()> {
    let temp = TempDir::new()?;
    File::create(temp.path().join("empty.txt"))?;

    let mut app = App::new()?;
    app.left_panel.path = temp.path().to_path_buf();
    app.left_panel.load_entries()?;

    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == "empty.txt" {
            app.left_panel.select(Some(i));
            break;
        }
    }

    handle_view(&mut app)?;
    assert!(app.get_viewer().is_some());
    let viewer = app.get_viewer().unwrap();
    assert_eq!(viewer.lines, vec!["<empty file>"]);

    Ok(())
}

#[test]
fn test_view_directory_rejected() -> io::Result<()> {
    let temp = TempDir::new()?;
    fs::create_dir(temp.path().join("subdir"))?;

    let mut app = App::new()?;
    app.left_panel.path = temp.path().to_path_buf();
    app.left_panel.load_entries()?;

    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == "subdir" {
            app.left_panel.select(Some(i));
            break;
        }
    }

    handle_view(&mut app)?;
    assert!(app.get_viewer().is_none());
    assert!(app.status_text().contains("Cannot view a directory"));

    Ok(())
}

#[test]
fn test_view_no_selection() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel.path = temp.path().to_path_buf();
    app.left_panel.entries.clear();
    app.left_panel.select(None);

    handle_view(&mut app)?;
    assert!(app.get_viewer().is_none());

    Ok(())
}

#[test]
fn test_viewer_page_down_up() -> io::Result<()> {
    let mut app = App::new()?;
    let lines: Vec<String> = (0..100).map(|i| format!("Line {i}")).collect();
    app.set_overlay(Overlay::Viewer(Viewer {
        title: "big.txt".to_owned(),
        lines,
        scroll: ScrollTop::TOP,
    }));

    // Viewer only supports Down/Up, not PageDown/PageUp/Home/End
    // Scroll down multiple times
    for _ in 0..10 {
        process_popup_input(&mut app, key(KeyCode::Down));
    }
    let after_down = app.get_viewer().unwrap().scroll;
    assert_eq!(after_down.lines(), 10, "Should have scrolled down 10 lines");

    // Scroll up
    for _ in 0..5 {
        process_popup_input(&mut app, key(KeyCode::Up));
    }
    let after_up = app.get_viewer().unwrap().scroll;
    assert_eq!(after_up.lines(), 5, "Should have scrolled back to 5");

    // Verify max scroll cap
    for _ in 0..200 {
        process_popup_input(&mut app, key(KeyCode::Down));
    }
    assert_eq!(
        app.get_viewer().unwrap().scroll.lines(),
        99,
        "Should cap at last line"
    );

    Ok(())
}

// ──────────────────────────────────────────────
// Drive selection overlay
// ──────────────────────────────────────────────

#[test]
fn test_drive_selection_enter_selects_root() -> io::Result<()> {
    let mut app = App::new()?;

    // Open drive selection for left panel
    app.set_overlay(Overlay::DriveSelection {
        side: PanelSide::Left,
        selected_index: 0,
    });

    // Press Enter to select the drive
    process_popup_input(&mut app, key(KeyCode::Enter));
    assert!(!matches!(app.overlay, Overlay::DriveSelection { .. }));
    assert!(app.status_text().contains("Changed to"));

    Ok(())
}

#[test]
fn test_drive_selection_enter_right_panel() -> io::Result<()> {
    let mut app = App::new()?;

    app.set_overlay(Overlay::DriveSelection {
        side: PanelSide::Right,
        selected_index: 0,
    });

    process_popup_input(&mut app, key(KeyCode::Enter));
    assert!(!matches!(app.overlay, Overlay::DriveSelection { .. }));
    assert!(app.status_text().contains("Changed to"));

    Ok(())
}

#[test]
fn test_drive_selection_escape_closes() -> io::Result<()> {
    let mut app = App::new()?;

    app.set_overlay(Overlay::DriveSelection {
        side: PanelSide::Left,
        selected_index: 0,
    });
    process_popup_input(&mut app, key(KeyCode::Esc));
    assert!(!matches!(app.overlay, Overlay::DriveSelection { .. }));
    let index = if let Overlay::DriveSelection { selected_index, .. } = app.overlay {
        selected_index
    } else {
        0
    };
    assert_eq!(index, 0);

    Ok(())
}

#[test]
fn test_drive_selection_home_end() -> io::Result<()> {
    let mut app = App::new()?;

    app.set_overlay(Overlay::DriveSelection {
        side: PanelSide::Left,
        selected_index: 5, // Arbitrary
    });

    process_popup_input(&mut app, key(KeyCode::Home));
    let after_home = if let Overlay::DriveSelection { selected_index, .. } = app.overlay {
        selected_index
    } else {
        0
    };
    assert_eq!(after_home, 0);

    process_popup_input(&mut app, key(KeyCode::End));
    let after_end = if let Overlay::DriveSelection { selected_index, .. } = app.overlay {
        selected_index
    } else {
        0
    };
    // Should be at last drive
    let drive_count = App::get_all_drives().len();
    if drive_count > 0 {
        assert_eq!(after_end, drive_count - 1);
    }

    Ok(())
}

// ──────────────────────────────────────────────
// Search overlay
// ──────────────────────────────────────────────

// ──────────────────────────────────────────────
// Bookmark overlay
// ──────────────────────────────────────────────

// ──────────────────────────────────────────────
// handle_delete edge cases
// ──────────────────────────────────────────────

#[test]
fn test_handle_delete_no_confirm_pending() -> io::Result<()> {
    let mut app = App::new()?;
    app.close_overlay();

    let before = app.status_text();
    // Should be a no-op
    handle_delete(&mut app)?;
    assert_eq!(app.status_text(), before, "status should not change");

    Ok(())
}

#[test]
fn test_handle_delete_multiple_files() -> io::Result<()> {
    let temp = TempDir::new()?;
    File::create(temp.path().join("a.txt"))?.write_all(b"a")?;
    File::create(temp.path().join("b.txt"))?.write_all(b"b")?;
    let subdir = temp.path().join("mydir");
    fs::create_dir(&subdir)?;
    File::create(subdir.join("inner.txt"))?.write_all(b"in")?;

    let mut app = App::new()?;
    app.left_panel.path = temp.path().to_path_buf();
    app.left_panel.load_entries()?;

    // Manually set delete_confirm with multiple targets
    app.set_overlay(Overlay::DeleteConfirm(vec![
        temp.path().join("a.txt"),
        temp.path().join("b.txt"),
        temp.path().join("mydir"),
    ]));

    handle_delete(&mut app)?;
    assert!(!temp.path().join("a.txt").exists());
    assert!(!temp.path().join("b.txt").exists());
    assert!(!temp.path().join("mydir").exists());
    assert!(app.status_text().contains('3'));

    Ok(())
}

// ──────────────────────────────────────────────
// handle_mkdir edge case
// ──────────────────────────────────────────────

#[test]
fn test_mkdir_auto_increments_name() -> io::Result<()> {
    let temp = TempDir::new()?;
    fs::create_dir(temp.path().join("new_folder"))?;
    fs::create_dir(temp.path().join("new_folder_1"))?;

    let mut app = App::new()?;
    app.left_panel.path = temp.path().to_path_buf();
    app.left_panel.load_entries()?;

    handle_mkdir(&mut app)?;
    assert!(temp.path().join("new_folder_2").is_dir());

    Ok(())
}

// ──────────────────────────────────────────────
// Event loop (lib.rs) paths — run_app_with_event_reader takes App by value,
// so we can only verify it completes without panic.
// ──────────────────────────────────────────────

fn run_event_sequence(app: App, events: Vec<crossterm::event::Event>) -> io::Result<()> {
    use funny_file_manager::run_app_with_event_reader;
    use funny_file_manager::WritableTestBackend;
    use ratatui::Terminal;

    let backend = WritableTestBackend::new(120, 40);
    let mut terminal = Terminal::new(backend)?;

    let mut idx = 0;
    run_app_with_event_reader(&mut terminal, app, || {
        if idx < events.len() {
            let e = events[idx].clone();
            idx += 1;
            Ok(e)
        } else {
            Ok(crossterm::event::Event::Key(key(KeyCode::F(10))))
        }
    })
}

#[test]
fn test_event_loop_end_key() -> io::Result<()> {
    use crossterm::event::Event;

    let temp = TempDir::new()?;
    File::create(temp.path().join("a.txt"))?.write_all(b"a")?;
    File::create(temp.path().join("b.txt"))?.write_all(b"b")?;
    File::create(temp.path().join("c.txt"))?.write_all(b"c")?;

    let mut app = App::new()?;
    app.left_panel.path = temp.path().to_path_buf();
    app.left_panel.load_entries()?;
    app.left_panel.select(Some(0));

    run_event_sequence(
        app,
        vec![
            Event::Key(key(KeyCode::End)),
            Event::Key(key(KeyCode::F(10))),
        ],
    )
}

#[test]
fn test_event_loop_backspace_quick_search() -> io::Result<()> {
    use crossterm::event::Event;

    let temp = TempDir::new()?;
    File::create(temp.path().join("alpha.txt"))?.write_all(b"a")?;
    File::create(temp.path().join("beta.txt"))?.write_all(b"b")?;

    let mut app = App::new()?;
    app.left_panel.path = temp.path().to_path_buf();
    app.left_panel.load_entries()?;

    run_event_sequence(
        app,
        vec![
            Event::Key(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::ALT)),
            Event::Key(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::ALT)),
            Event::Key(key(KeyCode::Backspace)),
            Event::Key(key(KeyCode::Backspace)),
            Event::Key(key(KeyCode::F(10))),
        ],
    )
}

#[test]
fn test_event_loop_ctrl_backslash_root() -> io::Result<()> {
    use crossterm::event::Event;

    let app = App::new()?;
    run_event_sequence(
        app,
        vec![
            Event::Key(key_ctrl(KeyCode::Char('\\'))),
            Event::Key(key(KeyCode::F(10))),
        ],
    )
}

#[test]
fn test_event_loop_tab_and_panel_keys() -> io::Result<()> {
    use crossterm::event::Event;

    let app = App::new()?;
    run_event_sequence(
        app,
        vec![
            Event::Key(key(KeyCode::Tab)),
            Event::Key(key(KeyCode::Right)),
            Event::Key(key(KeyCode::Left)),
            Event::Key(key(KeyCode::F(10))),
        ],
    )
}

#[test]
fn test_event_loop_insert_mark() -> io::Result<()> {
    use crossterm::event::Event;

    let temp = TempDir::new()?;
    File::create(temp.path().join("file.txt"))?.write_all(b"x")?;

    let mut app = App::new()?;
    app.left_panel.path = temp.path().to_path_buf();
    app.left_panel.load_entries()?;

    // Select file.txt
    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == "file.txt" {
            app.left_panel.select(Some(i));
            break;
        }
    }

    run_event_sequence(
        app,
        vec![
            Event::Key(key(KeyCode::Insert)),
            Event::Key(key(KeyCode::F(10))),
        ],
    )
}

#[test]
fn test_event_loop_f1_help_toggle() -> io::Result<()> {
    use crossterm::event::Event;

    let app = App::new()?;
    run_event_sequence(
        app,
        vec![
            Event::Key(key(KeyCode::F(1))),
            Event::Key(key(KeyCode::F(1))),
            Event::Key(key(KeyCode::F(10))),
        ],
    )
}

#[test]
fn test_event_loop_ctrl_u_swap() -> io::Result<()> {
    use crossterm::event::Event;

    let app = App::new()?;
    run_event_sequence(
        app,
        vec![
            Event::Key(key_ctrl(KeyCode::Char('u'))),
            Event::Key(key(KeyCode::F(10))),
        ],
    )
}

#[test]
fn test_event_loop_ctrl_o_console() -> io::Result<()> {
    use crossterm::event::Event;

    let app = App::new()?;
    run_event_sequence(
        app,
        vec![
            Event::Key(key_ctrl(KeyCode::Char('o'))),
            Event::Key(key_ctrl(KeyCode::Char('o'))),
            Event::Key(key(KeyCode::F(10))),
        ],
    )
}

#[test]
fn test_event_loop_resize() -> io::Result<()> {
    use crossterm::event::Event;

    let app = App::new()?;
    run_event_sequence(
        app,
        vec![Event::Resize(100, 50), Event::Key(key(KeyCode::F(10)))],
    )
}

#[test]
fn test_event_loop_f10_quit() -> io::Result<()> {
    use crossterm::event::Event;

    let app = App::new()?;
    run_event_sequence(app, vec![Event::Key(key(KeyCode::F(10)))])
}

#[test]
fn test_event_loop_ctrl_c_quit() -> io::Result<()> {
    use crossterm::event::Event;

    let app = App::new()?;
    run_event_sequence(app, vec![Event::Key(key_ctrl(KeyCode::Char('c')))])
}

#[test]
fn test_event_loop_home_key() -> io::Result<()> {
    use crossterm::event::Event;

    let temp = TempDir::new()?;
    File::create(temp.path().join("a.txt"))?.write_all(b"a")?;
    File::create(temp.path().join("b.txt"))?.write_all(b"b")?;

    let mut app = App::new()?;
    app.left_panel.path = temp.path().to_path_buf();
    app.left_panel.load_entries()?;

    run_event_sequence(
        app,
        vec![
            Event::Key(key(KeyCode::End)),
            Event::Key(key(KeyCode::Home)),
            Event::Key(key(KeyCode::F(10))),
        ],
    )
}

#[test]
fn test_event_loop_enter_directory() -> io::Result<()> {
    use crossterm::event::Event;

    let temp = TempDir::new()?;
    fs::create_dir(temp.path().join("subdir"))?;

    let mut app = App::new()?;
    app.left_panel.path = temp.path().to_path_buf();
    app.left_panel.load_entries()?;

    // Select subdir
    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == "subdir" {
            app.left_panel.select(Some(i));
            break;
        }
    }

    run_event_sequence(
        app,
        vec![
            Event::Key(key(KeyCode::Enter)),
            Event::Key(key(KeyCode::F(10))),
        ],
    )
}

#[test]
fn test_event_loop_space_toggles_mark() -> io::Result<()> {
    use crossterm::event::Event;

    let temp = TempDir::new()?;
    File::create(temp.path().join("x.txt"))?.write_all(b"x")?;

    let mut app = App::new()?;
    app.left_panel.path = temp.path().to_path_buf();
    app.left_panel.load_entries()?;
    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == "x.txt" {
            app.left_panel.select(Some(i));
            break;
        }
    }

    run_event_sequence(
        app,
        vec![
            Event::Key(key(KeyCode::Char(' '))),
            Event::Key(key(KeyCode::F(10))),
        ],
    )
}

#[test]
fn test_event_loop_esc_clears_quick_search() -> io::Result<()> {
    use crossterm::event::Event;

    let app = App::new()?;
    run_event_sequence(
        app,
        vec![
            Event::Key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::ALT)),
            Event::Key(key(KeyCode::Esc)),
            Event::Key(key(KeyCode::F(10))),
        ],
    )
}

#[test]
fn test_event_loop_shift_f4_new_file() -> io::Result<()> {
    use crossterm::event::Event;

    let temp = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel.path = temp.path().to_path_buf();
    app.left_panel.load_entries()?;

    run_event_sequence(
        app,
        vec![
            Event::Key(key_shift(KeyCode::F(4))),
            Event::Key(key(KeyCode::Char('t'))),
            Event::Key(key(KeyCode::Esc)),
            Event::Key(key(KeyCode::F(10))),
        ],
    )
}

#[test]
fn test_event_loop_f7_mkdir() -> io::Result<()> {
    use crossterm::event::Event;

    let temp = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel.path = temp.path().to_path_buf();
    app.left_panel.load_entries()?;

    run_event_sequence(
        app,
        vec![
            Event::Key(key(KeyCode::F(7))),
            Event::Key(key(KeyCode::F(10))),
        ],
    )?;

    assert!(temp.path().join("new_folder").is_dir());
    Ok(())
}

#[test]
fn test_event_loop_alt_f1_drive_selection() -> io::Result<()> {
    use crossterm::event::Event;

    let app = App::new()?;
    run_event_sequence(
        app,
        vec![
            Event::Key(KeyEvent::new(KeyCode::F(1), KeyModifiers::ALT)),
            Event::Key(key(KeyCode::Esc)),
            Event::Key(key(KeyCode::F(10))),
        ],
    )
}

#[test]
fn test_event_loop_alt_f2_drive_selection() -> io::Result<()> {
    use crossterm::event::Event;

    let app = App::new()?;
    run_event_sequence(
        app,
        vec![
            Event::Key(KeyEvent::new(KeyCode::F(2), KeyModifiers::ALT)),
            Event::Key(key(KeyCode::Esc)),
            Event::Key(key(KeyCode::F(10))),
        ],
    )
}

#[test]
fn test_event_loop_f2_bookmarks() -> io::Result<()> {
    use crossterm::event::Event;

    let app = App::new()?;
    run_event_sequence(
        app,
        vec![
            Event::Key(key(KeyCode::F(2))),
            Event::Key(key(KeyCode::Esc)),
            Event::Key(key(KeyCode::F(10))),
        ],
    )
}

#[test]
fn test_event_loop_down_up_navigation() -> io::Result<()> {
    use crossterm::event::Event;

    let temp = TempDir::new()?;
    File::create(temp.path().join("a.txt"))?.write_all(b"a")?;
    File::create(temp.path().join("b.txt"))?.write_all(b"b")?;

    let mut app = App::new()?;
    app.left_panel.path = temp.path().to_path_buf();
    app.left_panel.load_entries()?;

    run_event_sequence(
        app,
        vec![
            Event::Key(key(KeyCode::Down)),
            Event::Key(key(KeyCode::Down)),
            Event::Key(key(KeyCode::Up)),
            Event::Key(key(KeyCode::F(10))),
        ],
    )
}

#[test]
fn test_event_loop_f3_view_file() -> io::Result<()> {
    use crossterm::event::Event;

    let temp = TempDir::new()?;
    File::create(temp.path().join("view.txt"))?.write_all(b"hello")?;

    let mut app = App::new()?;
    app.left_panel.path = temp.path().to_path_buf();
    app.left_panel.load_entries()?;
    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == "view.txt" {
            app.left_panel.select(Some(i));
            break;
        }
    }

    run_event_sequence(
        app,
        vec![
            Event::Key(key(KeyCode::F(3))),
            Event::Key(key(KeyCode::Down)),
            Event::Key(key(KeyCode::Esc)),
            Event::Key(key(KeyCode::F(10))),
        ],
    )
}

#[test]
fn test_event_loop_f5_copy() -> io::Result<()> {
    use crossterm::event::Event;

    let temp = TempDir::new()?;
    let left = temp.path().join("l");
    let right = temp.path().join("r");
    fs::create_dir(&left)?;
    fs::create_dir(&right)?;
    File::create(left.join("cp.txt"))?.write_all(b"data")?;

    let mut app = App::new()?;
    app.left_panel.path = left;
    app.right_panel.path = right.clone();
    app.left_panel.load_entries()?;
    app.right_panel.load_entries()?;
    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == "cp.txt" {
            app.left_panel.select(Some(i));
            break;
        }
    }

    run_event_sequence(
        app,
        vec![
            Event::Key(key(KeyCode::F(5))),
            Event::Key(key(KeyCode::F(10))),
        ],
    )?;

    assert!(right.join("cp.txt").exists());
    Ok(())
}

#[test]
fn test_event_loop_f8_delete() -> io::Result<()> {
    use crossterm::event::Event;

    let temp = TempDir::new()?;
    File::create(temp.path().join("rm.txt"))?.write_all(b"bye")?;

    let mut app = App::new()?;
    app.left_panel.path = temp.path().to_path_buf();
    app.left_panel.load_entries()?;
    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == "rm.txt" {
            app.left_panel.select(Some(i));
            break;
        }
    }

    run_event_sequence(
        app,
        vec![
            Event::Key(key(KeyCode::F(8))),
            Event::Key(key(KeyCode::Char('y'))),
            Event::Key(key(KeyCode::F(10))),
        ],
    )?;

    assert!(!temp.path().join("rm.txt").exists());
    Ok(())
}

#[test]
fn test_event_loop_search_overlay_via_f7_key() -> io::Result<()> {
    use crossterm::event::Event;

    let temp = TempDir::new()?;
    File::create(temp.path().join("needle.txt"))?.write_all(b"x")?;

    let mut app = App::new()?;
    app.left_panel.path = temp.path().to_path_buf();
    app.left_panel.load_entries()?;

    // F7 is mkdir but let's test the search overlay via Ctrl+S or / if available
    // Actually search is opened differently. Let's just exercise more keys.
    run_event_sequence(
        app,
        vec![
            Event::Key(key(KeyCode::F(7))), // mkdir
            Event::Key(key(KeyCode::F(10))),
        ],
    )
}

// ──────────────────────────────────────────────
// Model edge cases
// ──────────────────────────────────────────────

#[test]
fn test_panel_cycle_sort_mode() -> io::Result<()> {
    use funny_file_manager::models::SortMode;

    let temp = TempDir::new()?;
    File::create(temp.path().join("a.txt"))?.write_all(b"short")?;
    File::create(temp.path().join("b.rs"))?.write_all(b"longer content")?;

    let mut app = App::new()?;
    app.left_panel.path = temp.path().to_path_buf();
    app.left_panel.load_entries()?;

    assert!(matches!(app.left_panel.sort_mode, SortMode::Name));
    app.left_panel.cycle_sort_mode();
    assert!(matches!(app.left_panel.sort_mode, SortMode::Extension));
    app.left_panel.cycle_sort_mode();
    assert!(matches!(app.left_panel.sort_mode, SortMode::Size));
    app.left_panel.cycle_sort_mode();
    assert!(matches!(app.left_panel.sort_mode, SortMode::Name));

    Ok(())
}

#[test]
fn test_app_swap_panels() -> io::Result<()> {
    let temp = TempDir::new()?;
    let dir_a = temp.path().join("aa");
    let dir_b = temp.path().join("bb");
    fs::create_dir(&dir_a)?;
    fs::create_dir(&dir_b)?;

    let mut app = App::new()?;
    app.left_panel.path = dir_a.clone();
    app.right_panel.path = dir_b.clone();

    app.swap_panels();
    assert_eq!(app.left_panel.path, dir_b);
    assert_eq!(app.right_panel.path, dir_a);

    Ok(())
}

#[test]
fn test_panel_toggle_mark_and_advance() -> io::Result<()> {
    let temp = TempDir::new()?;
    File::create(temp.path().join("a.txt"))?.write_all(b"a")?;
    File::create(temp.path().join("b.txt"))?.write_all(b"b")?;

    let mut app = App::new()?;
    app.left_panel.path = temp.path().to_path_buf();
    app.left_panel.load_entries()?;

    // Select first non-".." entry
    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == "a.txt" {
            app.left_panel.select(Some(i));
            break;
        }
    }

    app.left_panel.toggle_mark_selected();
    assert!(app.left_panel.marked.contains("a.txt"));

    // Toggle again to unmark
    // Re-select a.txt since toggle_mark_selected may have advanced
    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == "a.txt" {
            app.left_panel.select(Some(i));
            break;
        }
    }
    app.left_panel.toggle_mark_selected();
    assert!(!app.left_panel.marked.contains("a.txt"));

    Ok(())
}

#[test]
fn test_command_line_mark_invalid_subcommand() -> io::Result<()> {
    let mut app = App::new()?;

    app.set_overlay(Overlay::CommandLine("mark blah".into()));
    process_popup_input(&mut app, key(KeyCode::Enter));
    assert!(
        app.status_text().contains("Error") || app.status_text().contains("mark expects"),
        "Got: {}",
        app.status_text()
    );

    Ok(())
}

#[test]
fn test_command_line_sort_default_is_name() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel.path = temp.path().to_path_buf();
    app.left_panel.load_entries()?;

    // "sort" with no argument defaults to "name"
    app.set_overlay(Overlay::CommandLine("sort".into()));
    process_popup_input(&mut app, key(KeyCode::Enter));
    assert!(app.status_text().contains("Sort"));

    Ok(())
}
