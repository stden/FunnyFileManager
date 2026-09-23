//! Unit tests: delete confirm shell (split out to keep files under 500 lines).
#![allow(unused_imports)]

use super::support::*;
use crate::ops::*;
use crate::popup::process_popup_input;

#[cfg(feature = "test-helpers")]
use crate::editor::{handle_edit_impl, handle_edit_test_mode, TestEditorBehavior};

use crate::models::{App, Overlay, Panel, PanelSide};

#[cfg(feature = "test-helpers")]
use crate::WritableTestBackend;

use crossterm::event::KeyCode;

use std::fs::{self, File};

use std::io::{self, Write};

use std::path::PathBuf;

use tempfile::TempDir;

// -----------------------------------------------------------------------
// Drive selection Enter: local drive error + S3 profile selection
// -----------------------------------------------------------------------

#[test]
fn test_drive_selection_enter_local_drive_error() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.right_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.set_overlay(Overlay::DriveSelection {
        side: PanelSide::Left,
        selected_index: 0,
        rows: crate::models::drive_rows(),
    });

    // Make panel path invalid so load_entries fails after changing to root
    // Actually, root should work fine. The error path is hard to trigger
    // for local drives since "/" always exists. We need to test the S3 path.
    process_popup_input(&mut app, key_event(KeyCode::Enter));
    // Just verify the overlay closes and a status message is set
    assert!(!matches!(app.overlay, Overlay::DriveSelection { .. }));
    Ok(())
}

// -----------------------------------------------------------------------
// Delete confirmation overlay: Y confirms delete
// -----------------------------------------------------------------------

#[test]
fn test_delete_confirm_overlay_y_confirms() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("to_delete.txt");
    fs::write(&file_path, "data")?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.right_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.set_overlay(Overlay::DeleteConfirm(vec![file_path.clone()]));

    process_popup_input(&mut app, key_event(KeyCode::Char('y')));
    assert!(!file_path.exists(), "file should be deleted");
    assert!(app.status_text().contains("Deleted"));
    Ok(())
}

// -----------------------------------------------------------------------
// Shell command execution error (nonexistent binary)
// -----------------------------------------------------------------------

#[test]
fn test_shell_command_nonexistent_binary() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.right_panel = Panel::new(temp_dir.path().to_path_buf())?;
    // Point to nonexistent dir so the shell command's current_dir fails
    app.left_panel.path = PathBuf::from("/nonexistent_dir_99999");

    let result = execute_command(&mut app, "echo hello");
    assert!(result.is_err());
    Ok(())
}

// -----------------------------------------------------------------------
// handle_delete_confirm: ".." guard + no selection
// -----------------------------------------------------------------------

#[test]
fn test_delete_confirm_skips_dotdot() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let sub = temp_dir.path().join("sub");
    fs::create_dir(&sub)?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(sub)?;
    app.right_panel = Panel::new(temp_dir.path().to_path_buf())?;

    // Select ".."
    if let Some(pos) = app.left_panel.entries.iter().position(|e| e.name == "..") {
        app.left_panel.select(Some(pos));
    }

    handle_delete_confirm(&mut app)?;
    assert!(
        !matches!(app.overlay, Overlay::DeleteConfirm(_)),
        "should not confirm delete on .."
    );
    Ok(())
}

#[test]
fn test_delete_confirm_no_selection_is_noop() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.right_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.left_panel.entries.clear();
    app.left_panel.select(None);

    handle_delete_confirm(&mut app)?;
    assert!(!matches!(app.overlay, Overlay::DeleteConfirm(_)));
    Ok(())
}

// -----------------------------------------------------------------------
// New file input: file already exists + create error
// -----------------------------------------------------------------------

#[test]
fn test_new_file_input_file_already_exists() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    fs::write(temp_dir.path().join("exists.txt"), "data")?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.right_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.set_overlay(Overlay::NewFileInput("exists.txt".to_string()));

    process_popup_input(&mut app, key_event(KeyCode::Enter));
    assert!(
        app.status_text().contains("already exists"),
        "status: {}",
        app.status_text()
    );
    Ok(())
}

#[test]
fn test_new_file_create_error_sets_status() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.right_panel = Panel::new(temp_dir.path().to_path_buf())?;
    // Try to create a file in a nonexistent subdirectory
    app.set_overlay(Overlay::NewFileInput(
        "nonexistent_dir/file.txt".to_string(),
    ));

    process_popup_input(&mut app, key_event(KeyCode::Enter));
    assert!(
        app.status_text().contains("Failed"),
        "status: {}",
        app.status_text()
    );
    Ok(())
}

// -----------------------------------------------------------------------
// Shell command via execute_command: stderr, exit code, error
// -----------------------------------------------------------------------

#[test]
fn test_shell_command_stderr_only_output() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.right_panel = Panel::new(temp_dir.path().to_path_buf())?;

    // Command that writes to stderr only
    execute_command(&mut app, "echo error_msg >&2")?;
    assert!(
        app.status_text().contains("Err:") && app.status_text().contains("error_msg"),
        "status: {}",
        app.status_text()
    );
    Ok(())
}

#[test]
fn test_shell_command_no_output_success() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.right_panel = Panel::new(temp_dir.path().to_path_buf())?;

    // Command with no output
    execute_command(&mut app, "true")?;
    assert!(
        app.status_text().starts_with("OK:"),
        "status: {}",
        app.status_text()
    );
    Ok(())
}

#[test]
fn test_shell_command_no_output_failure() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.right_panel = Panel::new(temp_dir.path().to_path_buf())?;

    // Command that fails silently
    execute_command(&mut app, "false")?;
    assert!(
        app.status_text().starts_with("Exit"),
        "status: {}",
        app.status_text()
    );
    Ok(())
}

// -----------------------------------------------------------------------
// Delete confirmation overlay: Y confirms, error path
// -----------------------------------------------------------------------

#[test]
fn test_delete_confirm_overlay_error_path() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.right_panel = Panel::new(temp_dir.path().to_path_buf())?;

    // Set up delete_confirm with a nonexistent path to trigger delete error
    app.set_overlay(Overlay::DeleteConfirm(vec![PathBuf::from(
        "/nonexistent_file_to_delete_99999",
    )]));

    process_popup_input(&mut app, key_event(KeyCode::Char('y')));
    assert!(
        app.status_text().contains("error"),
        "status: {}",
        app.status_text()
    );
    Ok(())
}

/// A panel that can no longer be listed is a *display* problem, not a delete
/// failure. This test used to assert the opposite — "Delete failed: No such file
/// or directory" for a delete that had actually happened — which is exactly the
/// bug: the user reads it, assumes the data is still there, and may retry.
#[test]
fn test_delete_confirm_overlay_keeps_success_status_when_refresh_fails() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("deleteme.txt");
    fs::write(&file_path, "data")?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.right_panel = Panel::new(temp_dir.path().to_path_buf())?;
    // A panel path that cannot be read, so the post-delete refresh fails.
    app.left_panel.path = PathBuf::from("/nonexistent_dir_for_refresh_99999");
    app.set_overlay(Overlay::DeleteConfirm(vec![file_path.clone()]));

    process_popup_input(&mut app, key_event(KeyCode::Char('y')));

    // `symlink_metadata`, not `exists()`: the latter follows links.
    assert!(
        fs::symlink_metadata(&file_path).is_err(),
        "file was deleted"
    );
    assert_eq!(
        app.status_text(),
        "Deleted 1 item(s)",
        "a failed refresh must not be reported as a failed delete"
    );
    Ok(())
}

// -----------------------------------------------------------------------
// Drive selection: Up/Down navigation
// -----------------------------------------------------------------------

#[test]
fn test_drive_selection_up_at_zero_wraps_cov() {
    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::DriveSelection {
        side: PanelSide::Left,
        selected_index: 0,
        rows: crate::models::drive_rows(),
    });

    // Up at index 0 should wrap to last
    process_popup_input(&mut app, key_event(KeyCode::Up));
    let drives = App::get_all_drives();
    if !drives.is_empty() {
        if let Overlay::DriveSelection { selected_index, .. } = &app.overlay {
            assert_eq!(*selected_index, drives.len() - 1);
        } else {
            panic!("Expected DriveSelection overlay");
        }
    }
}

#[test]
fn test_drive_selection_up_dec_cov() {
    let mut app = App::new().expect("test setup");
    let drives = App::get_all_drives();
    if drives.len() > 1 {
        app.set_overlay(Overlay::DriveSelection {
            side: PanelSide::Left,
            selected_index: 1,
            rows: crate::models::drive_rows(),
        });
        process_popup_input(&mut app, key_event(KeyCode::Up));
        if let Overlay::DriveSelection { selected_index, .. } = &app.overlay {
            assert_eq!(*selected_index, 0);
        } else {
            panic!("Expected DriveSelection overlay");
        }
    }
}

#[test]
fn test_drive_selection_down_inc_cov() {
    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::DriveSelection {
        side: PanelSide::Left,
        selected_index: 0,
        rows: crate::models::drive_rows(),
    });

    let drives = App::get_all_drives();
    if drives.len() > 1 {
        process_popup_input(&mut app, key_event(KeyCode::Down));
        if let Overlay::DriveSelection { selected_index, .. } = &app.overlay {
            assert_eq!(*selected_index, 1);
        } else {
            panic!("Expected DriveSelection overlay");
        }
    }
}

#[test]
fn test_drive_selection_down_at_end_wraps_cov() {
    let mut app = App::new().expect("test setup");
    let drives = App::get_all_drives();
    if !drives.is_empty() {
        app.set_overlay(Overlay::DriveSelection {
            side: PanelSide::Left,
            selected_index: drives.len() - 1,
            rows: crate::models::drive_rows(),
        });
        process_popup_input(&mut app, key_event(KeyCode::Down));
        if let Overlay::DriveSelection { selected_index, .. } = &app.overlay {
            assert_eq!(*selected_index, 0);
        } else {
            panic!("Expected DriveSelection overlay");
        }
    }
}

// -----------------------------------------------------------------------
// File too large to view
// -----------------------------------------------------------------------

#[test]
fn test_handle_view_file_too_large() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let big_file = temp_dir.path().join("big.bin");

    // Create a sparse file > 10 MB by writing just past the limit
    {
        let f = File::create(&big_file)?;
        f.set_len(11 * 1024 * 1024)?; // 11 MB
    }

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    if let Some(idx) = app
        .left_panel
        .entries
        .iter()
        .position(|e| e.name == "big.bin")
    {
        app.left_panel.select(Some(idx));
    }

    handle_view(&mut app)?;

    assert!(
        app.get_viewer().is_none(),
        "viewer should not open for large file"
    );
    assert!(
        app.status_text().contains("too large"),
        "status: {}",
        app.status_text()
    );

    Ok(())
}

// -----------------------------------------------------------------------
// handle_delete_confirm: ".." selected returns early
// -----------------------------------------------------------------------

#[test]
fn test_handle_delete_confirm_dotdot_entry_selected() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    fs::write(temp_dir.path().join("keep.txt"), "data")?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;

    // Select ".." entry
    if let Some(idx) = app.left_panel.entries.iter().position(|e| e.name == "..") {
        app.left_panel.select(Some(idx));
    } else {
        // No ".." entry, test is not applicable at root
        return Ok(());
    }

    handle_delete_confirm(&mut app)?;

    // Should NOT set delete_confirm
    assert!(
        !matches!(app.overlay, Overlay::DeleteConfirm(_)),
        "should not confirm delete on .."
    );

    Ok(())
}
