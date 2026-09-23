//! Unit tests: source paths fallbacks (split out to keep files under 500 lines).
#![allow(unused_imports)]

use crate::models::FileName;

use crate::ops::file::get_source_paths;

use super::support::*;
use crate::ops::*;
use crate::popup::process_popup_input;

#[cfg(feature = "test-helpers")]
use crate::editor::{handle_edit_impl, handle_edit_test_mode, TestEditorBehavior};

use crate::models::{App, Overlay, Panel, PanelSide};

#[cfg(feature = "test-helpers")]
use crate::WritableTestBackend;

use crossterm::event::KeyCode;

use std::fs::{self};

use std::io::{self, Write};

use tempfile::TempDir;

#[test]
fn test_get_source_paths_no_marks_uses_selected() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    fs::write(temp_dir.path().join("only.txt"), "data")?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;

    // Select the file
    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == "only.txt" {
            app.left_panel.select(Some(i));
            break;
        }
    }

    let paths = get_source_paths(&app);
    assert_eq!(paths.len(), 1);
    assert!(paths[0].ends_with("only.txt"));

    Ok(())
}

#[test]
fn test_get_source_paths_no_marks_no_selection() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.left_panel.entries.clear();
    app.left_panel.select(None);

    let paths = get_source_paths(&app);
    assert!(paths.is_empty());

    Ok(())
}

// -----------------------------------------------------------------------
// handle_copy / handle_move with directories
// -----------------------------------------------------------------------

#[test]
fn test_handle_copy_directory_recursive() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let src_dir = temp_dir.path().join("source");
    let target = temp_dir.path().join("target");
    fs::create_dir(&src_dir)?;
    fs::create_dir(&target)?;
    fs::write(src_dir.join("inside.txt"), "data")?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.right_panel = Panel::new(target)?;

    // Select the source directory
    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == "source" && entry.kind.is_dir() {
            app.left_panel.select(Some(i));
            break;
        }
    }

    handle_copy(&mut app)?;
    // Verify the directory was copied to the target
    assert!(app.right_panel.path.join("source").exists());
    assert!(app
        .right_panel
        .path
        .join("source")
        .join("inside.txt")
        .exists());

    Ok(())
}

#[test]
fn test_handle_move_with_rename_fallback() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let target = temp_dir.path().join("target");
    fs::create_dir(&target)?;
    fs::write(temp_dir.path().join("moveme.txt"), "data")?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.right_panel = Panel::new(target.clone())?;

    // Select the file
    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == "moveme.txt" {
            app.left_panel.select(Some(i));
            break;
        }
    }

    // Force rename failure to exercise copy+delete fallback
    let guard = ForceRenameFailureGuard::enable();
    let result = handle_move(&mut app);
    drop(guard);

    result?;
    assert!(target.join("moveme.txt").exists());
    assert!(!temp_dir.path().join("moveme.txt").exists());

    Ok(())
}

#[test]
fn test_handle_move_dir_with_rename_fallback() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let src_dir = temp_dir.path().join("movedir");
    let target = temp_dir.path().join("target");
    fs::create_dir(&src_dir)?;
    fs::create_dir(&target)?;
    fs::write(src_dir.join("child.txt"), "data")?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.right_panel = Panel::new(target.clone())?;

    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == "movedir" && entry.kind.is_dir() {
            app.left_panel.select(Some(i));
            break;
        }
    }

    let guard = ForceRenameFailureGuard::enable();
    let result = handle_move(&mut app);
    drop(guard);

    result?;
    assert!(target.join("movedir").join("child.txt").exists());
    assert!(!src_dir.exists());

    Ok(())
}

// -----------------------------------------------------------------------
// handle_delete with partial errors
// -----------------------------------------------------------------------

#[test]
fn test_handle_delete_with_nonexistent_file() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;

    // Set up delete_confirm with a file that doesn't exist
    app.set_overlay(Overlay::DeleteConfirm(vec![temp_dir
        .path()
        .join("ghost.txt")]));
    handle_delete(&mut app)?;

    assert!(
        app.status_text().contains("error"),
        "should report error for nonexistent file: {}",
        app.status_text()
    );

    Ok(())
}

#[test]
fn test_handle_delete_mixed_success_and_error() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let real_file = temp_dir.path().join("real.txt");
    fs::write(&real_file, "data")?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;

    // One real file + one nonexistent
    app.set_overlay(Overlay::DeleteConfirm(vec![
        real_file.clone(),
        temp_dir.path().join("ghost.txt"),
    ]));
    handle_delete(&mut app)?;

    assert!(!real_file.exists(), "real file should be deleted");
    assert!(
        app.status_text().contains("1 error"),
        "should report partial error: {}",
        app.status_text()
    );
    assert!(
        app.status_text().contains("Deleted 1"),
        "should report 1 deleted: {}",
        app.status_text()
    );

    Ok(())
}

#[test]
fn test_handle_delete_no_confirm_is_noop() -> io::Result<()> {
    let mut app = App::new()?;
    app.close_overlay();

    handle_delete(&mut app)?;
    // Should just return Ok without doing anything

    Ok(())
}

// -----------------------------------------------------------------------
// handle_copy / handle_move with multiple marked files
// -----------------------------------------------------------------------

#[test]
fn test_handle_copy_multiple_marked() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let target = temp_dir.path().join("dest");
    fs::create_dir(&target)?;
    fs::write(temp_dir.path().join("a.txt"), "a")?;
    fs::write(temp_dir.path().join("b.txt"), "b")?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.right_panel = Panel::new(target.clone())?;
    app.left_panel.marked.insert(FileName::from("a.txt"));
    app.left_panel.marked.insert(FileName::from("b.txt"));

    handle_copy(&mut app)?;
    assert!(target.join("a.txt").exists());
    assert!(target.join("b.txt").exists());
    assert!(
        app.status_text().contains("2 item(s)"),
        "should report 2 items: {}",
        app.status_text()
    );

    Ok(())
}

#[test]
fn test_handle_move_empty_sources_is_noop() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.right_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.left_panel.entries.clear();
    app.left_panel.select(None);

    handle_move(&mut app)?;
    // Should just return Ok

    Ok(())
}

// -----------------------------------------------------------------------
// execute_command: error propagation through command line
// -----------------------------------------------------------------------

#[test]
fn test_execute_command_error_sets_status() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.right_panel = Panel::new(temp_dir.path().to_path_buf())?;

    // Enter command line mode, type an invalid command, submit
    app.set_overlay(Overlay::CommandLine("cd /nonexistent_path_99999".into()));
    let key = key_event(KeyCode::Enter);
    process_popup_input(&mut app, key);
    assert!(
        app.status_text().contains("Error"),
        "status should contain error: {}",
        app.status_text()
    );
    Ok(())
}

// -----------------------------------------------------------------------
// Drive selection: Up wraps, Down wraps
// -----------------------------------------------------------------------

#[test]
fn test_drive_selection_up_wraps_around() -> io::Result<()> {
    let mut app = App::new()?;
    app.set_overlay(Overlay::DriveSelection {
        side: PanelSide::Left,
        selected_index: 0,
        rows: crate::models::drive_rows(),
    });

    // Up from 0 should wrap to last
    process_popup_input(&mut app, key_event(KeyCode::Up));
    let drives = App::get_all_drives();
    if let Overlay::DriveSelection { selected_index, .. } = &app.overlay {
        assert_eq!(*selected_index, drives.len() - 1);
    } else {
        panic!("Expected DriveSelection overlay");
    }
    Ok(())
}

#[test]
fn test_drive_selection_down_wraps_around() -> io::Result<()> {
    let mut app = App::new()?;
    let drives = App::get_all_drives();
    app.set_overlay(Overlay::DriveSelection {
        side: PanelSide::Left,
        selected_index: drives.len() - 1,
        rows: crate::models::drive_rows(),
    });

    // Down from last should wrap to 0
    process_popup_input(&mut app, key_event(KeyCode::Down));
    if let Overlay::DriveSelection { selected_index, .. } = &app.overlay {
        assert_eq!(*selected_index, 0);
    } else {
        panic!("Expected DriveSelection overlay");
    }
    Ok(())
}

// -----------------------------------------------------------------------
// Drive selection: Enter on local drive + S3 profile
// -----------------------------------------------------------------------

#[test]
fn test_drive_selection_enter_local_drive() -> io::Result<()> {
    let mut app = App::new()?;
    let temp_dir = TempDir::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.right_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.set_overlay(Overlay::DriveSelection {
        side: PanelSide::Left,
        selected_index: 0,
        rows: crate::models::drive_rows(),
    });

    process_popup_input(&mut app, key_event(KeyCode::Enter));
    assert!(
        !matches!(app.overlay, Overlay::DriveSelection { .. }),
        "drives overlay should close"
    );
    assert!(
        app.status_text().contains("Changed") || app.status_text().contains("S3"),
        "should show status: {}",
        app.status_text()
    );
    Ok(())
}

#[test]
fn test_drive_selection_enter_s3_profile_overlay() -> io::Result<()> {
    let mut app = App::new()?;
    let drives = App::get_all_drives();
    // Find an S3 profile entry if it exists
    if let Some(pos) = drives
        .iter()
        .position(|d| matches!(d, crate::models::DriveEntry::S3Profile(_)))
    {
        app.set_overlay(Overlay::DriveSelection {
            side: PanelSide::Left,
            selected_index: pos,
            rows: crate::models::drive_rows(),
        });
        process_popup_input(&mut app, key_event(KeyCode::Enter));
        assert!(app.status_text().contains("S3"));
    }
    Ok(())
}

// -----------------------------------------------------------------------
// Drive selection: non-wrapping Up and Down
// -----------------------------------------------------------------------

#[test]
fn test_drive_selection_up_decrements() -> io::Result<()> {
    let mut app = App::new()?;
    let drives = App::get_all_drives();
    if drives.len() >= 2 {
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
    Ok(())
}

#[test]
fn test_drive_selection_down_increments() -> io::Result<()> {
    let mut app = App::new()?;
    let drives = App::get_all_drives();
    if drives.len() >= 2 {
        app.set_overlay(Overlay::DriveSelection {
            side: PanelSide::Left,
            selected_index: 0,
            rows: crate::models::drive_rows(),
        });
        process_popup_input(&mut app, key_event(KeyCode::Down));
        if let Overlay::DriveSelection { selected_index, .. } = &app.overlay {
            assert_eq!(*selected_index, 1);
        } else {
            panic!("Expected DriveSelection overlay");
        }
    }
    Ok(())
}
