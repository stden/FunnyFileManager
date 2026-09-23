//! Unit tests: new file and drives (split out to keep files under 500 lines).

use crate::models::FileName;

use crate::ops::file::format_operation_status;

use super::support::*;
use crate::ops::*;
use crate::popup::process_popup_input;

use crate::models::{App, BookmarkSlot, Overlay, Panel};

use crossterm::event::KeyCode;

use std::fs::{self, File};

use std::io;

use std::path::Path;

use tempfile::TempDir;

#[test]
fn test_execute_command_bookmark_go_empty_slot_returns_error() -> io::Result<()> {
    let mut app = App::new().expect("test setup");
    // Slot 5 is empty by default.
    assert!(app
        .bookmarks
        .get(BookmarkSlot::from_usize(5).expect("slot"))
        .is_none());

    let result = execute_command(&mut app, "bookmark go 5");
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("empty") || msg.contains("bookmark"),
        "error message should reference bookmark"
    );

    Ok(())
}

// --- format_operation_status ---

#[test]
fn test_format_operation_status_no_source() {
    let status = format_operation_status(0, "Copied", None);
    assert!(status.is_empty());
}

// -----------------------------------------------------------------------
// New-file input overlay -- Enter with already-existing file sets error msg
// but keeps the new_file_input set to None (the overlay is cleared)
// -----------------------------------------------------------------------

#[test]
fn test_new_file_input_enter_existing_file_sets_error_message() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let existing = temp_dir.path().join("exists.txt");
    File::create(&existing).expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.left_panel.load_entries().expect("test setup");
    app.set_overlay(Overlay::NewFileInput("exists.txt".to_owned()));

    process_popup_input(&mut app, key_event(KeyCode::Enter));

    // Overlay is cleared.
    assert!(
        app.get_new_file_input_mut().is_none(),
        "overlay should be cleared"
    );
    assert!(
        app.status_text().contains("already exists"),
        "status should note that file already exists"
    );

    Ok(())
}

#[test]
fn test_handle_mkdir_first_time() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    handle_mkdir(&mut app).expect("test setup");

    assert!(temp_dir.path().join("new_folder").exists());
    assert!(app.status_text().contains("Created directory"));

    Ok(())
}

#[test]
fn test_new_file_input_enter_creates_new_file() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.left_panel.load_entries().expect("test setup");
    app.set_overlay(Overlay::NewFileInput("brand_new.txt".to_owned()));

    process_popup_input(&mut app, key_event(KeyCode::Enter));

    assert!(
        !matches!(app.overlay, Overlay::NewFileInput(_)),
        "overlay should be cleared after creation"
    );
    assert!(
        temp_dir.path().join("brand_new.txt").exists(),
        "file should have been created"
    );
    assert!(
        app.status_text().contains("Created file"),
        "status should confirm creation"
    );

    Ok(())
}

// --- find_and_select_entry skips dotdot ---

// -----------------------------------------------------------------------
// Drive selection overlay -- Enter navigates the right panel
// -----------------------------------------------------------------------

#[test]
fn test_drive_selection_enter_navigates_right_panel() -> io::Result<()> {
    let mut app = App::new().expect("test setup");
    // Target the right panel.
    app.set_overlay(Overlay::DriveSelection {
        side: crate::models::PanelSide::Right,
        selected_index: 0,
        rows: crate::models::drive_rows(),
    });

    let drives = App::get_all_drives();
    if drives.is_empty() {
        return Ok(());
    }

    // Only proceed when the first drive is a local path that exists.
    let first_local_path = match &drives[0] {
        crate::models::DriveEntry::Local(p) if p.exists() => p.clone(),
        _ => return Ok(()),
    };

    // A drive remembers its current directory, as in Far: when the active
    // (left) panel is already on this drive, the right panel opens the same
    // folder rather than the root.
    let expected = if app.left_panel.path.starts_with(&first_local_path) {
        app.left_panel.path.clone()
    } else {
        first_local_path
    };

    process_popup_input(&mut app, key_event(KeyCode::Enter));

    assert!(
        !matches!(app.overlay, Overlay::DriveSelection { .. }),
        "drive overlay should close after Enter"
    );
    assert_eq!(
        app.right_panel.path, expected,
        "right panel should open the selected drive's current directory"
    );

    Ok(())
}

// -----------------------------------------------------------------------
// Command-line overlay -- Ctrl+Char is ignored (char not appended)
// -----------------------------------------------------------------------

#[test]
fn test_command_line_char_appends_to_buffer() -> io::Result<()> {
    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::CommandLine("he".into()));

    process_popup_input(&mut app, key_event(KeyCode::Char('l')));

    assert_eq!(
        app.get_command_line(),
        Some("hel"),
        "plain char should be appended to command buffer"
    );

    Ok(())
}

#[test]
fn test_command_line_backspace_on_single_char_leaves_empty() -> io::Result<()> {
    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::CommandLine("x".into()));

    process_popup_input(&mut app, key_event(KeyCode::Backspace));

    assert_eq!(
        app.get_command_line(),
        Some(""),
        "backspace on single char should leave Some(\"\"), not None"
    );

    Ok(())
}

// -----------------------------------------------------------------------
// Shell command execution -- stdout / stderr / success / exit code branches
// -----------------------------------------------------------------------

#[test]
fn test_shell_command_stdout_output() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.right_panel = Panel::new(temp_dir.path().to_path_buf())?;

    // "echo hello" produces stdout
    execute_command(&mut app, "echo hello")?;
    assert_eq!(app.status_text(), "hello");

    Ok(())
}

#[test]
fn test_shell_command_stdout_multiline_uses_last() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.right_panel = Panel::new(temp_dir.path().to_path_buf())?;

    // printf produces multi-line stdout; status should be the last line
    execute_command(&mut app, "printf 'line1\\nline2\\nline3'")?;
    assert_eq!(app.status_text(), "line3");

    Ok(())
}

#[test]
fn test_shell_command_stderr_output() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.right_panel = Panel::new(temp_dir.path().to_path_buf())?;

    // Command that writes to stderr but no stdout
    execute_command(&mut app, "echo errmsg >&2 && true")?;
    assert!(
        app.status_text().contains("Err:"),
        "stderr output should be prefixed with 'Err:': {}",
        app.status_text()
    );
    assert!(
        app.status_text().contains("errmsg"),
        "stderr content should appear in status: {}",
        app.status_text()
    );

    Ok(())
}

#[test]
fn test_shell_command_success_no_output() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.right_panel = Panel::new(temp_dir.path().to_path_buf())?;

    // "true" produces no output and exits 0
    execute_command(&mut app, "true")?;
    assert!(
        app.status_text().starts_with("OK:"),
        "success with no output should show 'OK: ...': {}",
        app.status_text()
    );

    Ok(())
}

#[test]
fn test_shell_command_exit_code_failure() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.right_panel = Panel::new(temp_dir.path().to_path_buf())?;

    // "false" produces no output and exits non-zero
    execute_command(&mut app, "false")?;
    assert!(
        app.status_text().starts_with("Exit"),
        "failure with no output should show 'Exit ...': {}",
        app.status_text()
    );

    Ok(())
}

// -----------------------------------------------------------------------
// handle_view edge cases
// -----------------------------------------------------------------------

#[test]
fn test_handle_view_empty_file_shows_placeholder() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;
    fs::write(temp_dir.path().join("empty.txt"), "")?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;

    // Select the empty file
    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == "empty.txt" {
            app.left_panel.select(Some(i));
            break;
        }
    }

    handle_view(&mut app)?;
    let viewer = app.get_viewer().expect("viewer should be open");
    assert_eq!(viewer.lines.len(), 1);
    assert_eq!(viewer.lines[0], "<empty file>");

    Ok(())
}

#[test]
fn test_handle_view_directory_sets_status() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;
    fs::create_dir(temp_dir.path().join("mydir"))?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;

    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == "mydir" && entry.kind.is_dir() {
            app.left_panel.select(Some(i));
            break;
        }
    }

    handle_view(&mut app)?;
    assert!(
        app.get_viewer().is_none(),
        "viewer should not open for directory"
    );
    assert!(
        matches!(app.overlay, Overlay::Information(_)),
        "F3 on a directory measures it, as Far does"
    );

    Ok(())
}

// -----------------------------------------------------------------------
// handle_delete_confirm -- ".." selected should be a no-op
// -----------------------------------------------------------------------

#[test]
fn test_handle_delete_confirm_dotdot_is_noop() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let subdir = temp_dir.path().join("sub");
    fs::create_dir(&subdir)?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(subdir)?;

    // ".." should be the first entry
    assert_eq!(app.left_panel.entries[0].name, "..");
    app.left_panel.select(Some(0));

    handle_delete_confirm(&mut app)?;
    assert!(
        !matches!(app.overlay, Overlay::DeleteConfirm(_)),
        "delete confirm should not activate for '..'"
    );

    Ok(())
}

// -----------------------------------------------------------------------
// format_operation_status branches
// -----------------------------------------------------------------------

#[test]
fn test_format_operation_status_multiple_items() {
    let result = format_operation_status(3, "Copied", Some(Path::new("/tmp/file.txt")));
    assert_eq!(result, "Copied 3 item(s)");
}

#[test]
fn test_format_operation_status_single_item() {
    let dest = Path::new("/tmp/target/file.txt");
    let result = format_operation_status(1, "Moved", Some(dest));
    assert!(
        result.starts_with("Moved to"),
        "single item should show destination: {result}"
    );
    assert!(result.contains("file.txt"));
}

#[test]
fn test_format_operation_status_no_source_empty() {
    let result = format_operation_status(0, "Copied", None);
    assert_eq!(result, "");
}

// -----------------------------------------------------------------------
// get_source_paths branches
// -----------------------------------------------------------------------

#[test]
fn test_get_source_paths_with_marks() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    fs::write(temp_dir.path().join("a.txt"), "a")?;
    fs::write(temp_dir.path().join("b.txt"), "b")?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.left_panel.marked.insert(FileName::from("a.txt"));
    app.left_panel.marked.insert(FileName::from("b.txt"));

    let paths = get_source_paths(&app);
    assert_eq!(paths.len(), 2);

    Ok(())
}
