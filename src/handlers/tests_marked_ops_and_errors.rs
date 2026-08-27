//! Unit tests: marked ops and errors (split out to keep files under 500 lines).
#![allow(unused_imports)]

use super::tests::*;
use crate::models::EntryKind;
use crate::models::FileName;
use crate::models::FileSize;

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
fn test_handle_view_error_paths() -> io::Result<()> {
    let temp_dir_outer = TempDir::new().expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir_outer.path().to_path_buf())?;
    app.right_panel = Panel::new(temp_dir_outer.path().to_path_buf())?;

    // No selection
    handle_view(&mut app).expect("test setup");
    assert!(app.get_viewer().is_none());

    // Use the existing temp_dir_outer
    app.left_panel.path = temp_dir_outer.path().to_path_buf();

    // Non-existent directory (selected but missing on disk)
    app.left_panel.entries.clear();
    app.left_panel.entries.push(crate::models::DirEntry {
        name: "..".to_string(),
        kind: EntryKind::Directory,
        size: FileSize::from_bytes(0),
    });
    app.left_panel.entries.push(crate::models::DirEntry {
        name: "missing.txt".to_string(),
        kind: EntryKind::File,
        size: FileSize::from_bytes(0),
    });
    app.left_panel.select(Some(1));
    // This should fail because missing.txt doesn't exist on disk
    assert!(handle_view(&mut app).is_err());

    // View a directory (that exists)
    let subdir = temp_dir_outer.path().join("subdir");
    std::fs::create_dir(&subdir).expect("test setup");
    app.left_panel.load_entries().expect("test setup");

    // Find subdir in entries and select it
    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == "subdir" {
            app.left_panel.select(Some(i));
            break;
        }
    }

    handle_view(&mut app).expect("test setup");
    assert!(app.status_text().contains("Cannot view a directory"));

    Ok(())
}

#[test]
fn test_handle_copy_no_selection_logic() -> io::Result<()> {
    let mut app = App::new().expect("test setup");
    app.left_panel.entries.clear();
    app.left_panel.select(None);

    // Should return Ok(()) and do nothing
    handle_copy(&mut app).expect("test setup");

    Ok(())
}

// --- Multi-file marked operations ---

#[test]
fn test_handle_copy_multiple_marked_files() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let left_dir = temp_dir.path().join("left");
    let right_dir = temp_dir.path().join("right");
    fs::create_dir(&left_dir).expect("test setup");
    fs::create_dir(&right_dir).expect("test setup");

    fs::write(left_dir.join("a.txt"), b"aaa").expect("test setup");
    fs::write(left_dir.join("b.txt"), b"bbb").expect("test setup");
    fs::write(left_dir.join("c.txt"), b"ccc").expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(left_dir).expect("test setup");
    app.right_panel = Panel::new(right_dir.clone()).expect("test setup");

    // Mark multiple files
    app.left_panel.marked.insert(FileName::from("a.txt"));
    app.left_panel.marked.insert(FileName::from("b.txt"));

    handle_copy(&mut app).expect("test setup");

    assert!(right_dir.join("a.txt").exists());
    assert!(right_dir.join("b.txt").exists());
    // count > 1 branch in format_operation_status
    assert!(app.status_text().contains("Copied"));
    assert!(app.status_text().contains('2'));

    Ok(())
}

#[test]
fn test_handle_move_multiple_marked_files() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let left_dir = temp_dir.path().join("left");
    let right_dir = temp_dir.path().join("right");
    fs::create_dir(&left_dir).expect("test setup");
    fs::create_dir(&right_dir).expect("test setup");

    fs::write(left_dir.join("x.txt"), b"xxx").expect("test setup");
    fs::write(left_dir.join("y.txt"), b"yyy").expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(left_dir.clone()).expect("test setup");
    app.right_panel = Panel::new(right_dir.clone()).expect("test setup");

    app.left_panel.marked.insert(FileName::from("x.txt"));
    app.left_panel.marked.insert(FileName::from("y.txt"));

    handle_move(&mut app).expect("test setup");

    assert!(!left_dir.join("x.txt").exists());
    assert!(!left_dir.join("y.txt").exists());
    assert!(right_dir.join("x.txt").exists());
    assert!(right_dir.join("y.txt").exists());
    assert!(app.status_text().contains("Moved"));
    assert!(app.status_text().contains('2'));

    Ok(())
}

// --- Move with forced rename failure (directory fallback) ---

#[test]
fn test_handle_move_dir_fallback_on_rename_failure() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let left_dir = temp_dir.path().join("left");
    let right_dir = temp_dir.path().join("right");
    fs::create_dir(&left_dir).expect("test setup");
    fs::create_dir(&right_dir).expect("test setup");

    let source_dir = left_dir.join("mydir");
    fs::create_dir(&source_dir).expect("test setup");
    fs::write(source_dir.join("inner.txt"), b"data").expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(left_dir).expect("test setup");
    app.right_panel = Panel::new(right_dir.clone()).expect("test setup");

    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == "mydir" {
            app.left_panel.select(Some(i));
            break;
        }
    }

    let _guard = ForceRenameFailureGuard::enable();
    handle_move(&mut app).expect("test setup");

    // Should have been copy+delete
    assert!(!source_dir.exists());
    assert!(right_dir.join("mydir").exists());
    assert!(right_dir.join("mydir").join("inner.txt").exists());

    Ok(())
}

// --- Delete edge cases ---

#[test]
fn test_handle_delete_no_confirm_set() -> io::Result<()> {
    let mut app = App::new().expect("test setup");
    // No longer exists: app.delete_confirm = None;

    // Should return Ok(()) without doing anything
    handle_delete(&mut app).expect("test setup");

    Ok(())
}

#[test]
fn test_handle_delete_with_partial_errors() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let existing = temp_dir.path().join("exists.txt");
    fs::write(&existing, b"data").expect("test setup");

    let nonexistent = temp_dir.path().join("gone.txt");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.set_overlay(Overlay::DeleteConfirm(vec![existing.clone(), nonexistent]));

    handle_delete(&mut app).expect("test setup");

    assert!(!existing.exists());
    // Should report both deleted count and errors
    assert!(app.status_text().contains("1 item(s)"));
    assert!(app.status_text().contains("error"));

    Ok(())
}

#[test]
fn test_handle_delete_confirm_dotdot_selected() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let subdir = temp_dir.path().join("sub");
    fs::create_dir(&subdir).expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(subdir).expect("test setup");
    // Select ".." (should be index 0)
    app.left_panel.select(Some(0));

    handle_delete_confirm(&mut app).expect("test setup");
    // Should not set delete_confirm for ".."
    assert!(!matches!(app.overlay, Overlay::DeleteConfirm(_)));

    Ok(())
}

#[test]
fn test_handle_delete_confirm_marked_files() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    fs::write(temp_dir.path().join("a.txt"), b"a").expect("test setup");
    fs::write(temp_dir.path().join("b.txt"), b"b").expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    app.left_panel.marked.insert(FileName::from("a.txt"));
    app.left_panel.marked.insert(FileName::from("b.txt"));

    handle_delete_confirm(&mut app).expect("test setup");

    if let Overlay::DeleteConfirm(paths) = &app.overlay {
        assert_eq!(paths.len(), 2);
    } else {
        panic!("Expected DeleteConfirm overlay");
    }

    Ok(())
}

// --- Drive selection overlay edge cases ---

// -----------------------------------------------------------------------
// Drive selection overlay – additional branch coverage
// -----------------------------------------------------------------------

/// Up key at index 0 wraps to the last drive.
#[test]
fn test_drive_selection_up_wraps_to_last() -> io::Result<()> {
    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::DriveSelection {
        side: PanelSide::Left,
        selected_index: 0,
    });

    let drive_count = crate::models::App::get_all_drives().len();
    if drive_count == 0 {
        return Ok(());
    }

    process_popup_input(&mut app, key_event(KeyCode::Up));
    if let Overlay::DriveSelection { selected_index, .. } = &app.overlay {
        assert_eq!(*selected_index, drive_count - 1);
    }

    Ok(())
}

/// Down key at last index wraps to 0.
#[test]
fn test_drive_selection_down_wraps_to_first() -> io::Result<()> {
    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::DriveSelection {
        side: PanelSide::Left,
        selected_index: 0,
    });

    let drive_count = crate::models::App::get_all_drives().len();
    if drive_count == 0 {
        return Ok(());
    }

    app.set_overlay(Overlay::DriveSelection {
        side: PanelSide::Left,
        selected_index: drive_count - 1,
    });
    process_popup_input(&mut app, key_event(KeyCode::Down));
    if let Overlay::DriveSelection { selected_index, .. } = &app.overlay {
        assert_eq!(*selected_index, 0);
    }

    Ok(())
}

#[test]
fn test_process_popup_input_drive_selection_s3_profile() -> io::Result<()> {
    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::DriveSelection {
        side: PanelSide::Right,
        selected_index: 0,
    });

    let drives = App::get_all_drives();
    // Find S3 profile entry if any
    let s3_index = drives
        .iter()
        .position(|d| matches!(d, crate::models::DriveEntry::S3Profile(_)));

    if let Some(idx) = s3_index {
        app.set_overlay(Overlay::DriveSelection {
            side: PanelSide::Right,
            selected_index: idx,
        });
        assert!(process_popup_input(&mut app, key_event(KeyCode::Enter)));
        assert!(app.status_text().contains("S3 profile"));
        assert!(!matches!(app.overlay, Overlay::DriveSelection { .. }));
    }

    Ok(())
}

#[test]
fn test_process_popup_input_drive_selection_wrap_up() -> io::Result<()> {
    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::DriveSelection {
        side: PanelSide::Left,
        selected_index: 0,
    });

    // Up at 0 should wrap to last
    assert!(process_popup_input(&mut app, key_event(KeyCode::Up)));
    let drives = App::get_all_drives();
    if !drives.is_empty() {
        if let Overlay::DriveSelection { selected_index, .. } = &app.overlay {
            assert_eq!(*selected_index, drives.len() - 1);
        }
    }

    Ok(())
}

#[test]
fn test_process_popup_input_drive_selection_wrap_down() -> io::Result<()> {
    let mut app = App::new().expect("test setup");
    let drives = App::get_all_drives();
    let last_idx = if drives.is_empty() {
        0
    } else {
        drives.len() - 1
    };
    app.set_overlay(Overlay::DriveSelection {
        side: PanelSide::Left,
        selected_index: last_idx,
    });

    // Down at last should wrap to 0
    assert!(process_popup_input(&mut app, key_event(KeyCode::Down)));
    if let Overlay::DriveSelection { selected_index, .. } = &app.overlay {
        assert_eq!(*selected_index, 0);
    }

    Ok(())
}

/// Home key jumps to index 0.
#[test]
fn test_drive_selection_home() -> io::Result<()> {
    let mut app = App::new().expect("test setup");
    let drive_count = crate::models::App::get_all_drives().len();
    let last_idx = if drive_count == 0 { 0 } else { drive_count - 1 };
    app.set_overlay(Overlay::DriveSelection {
        side: PanelSide::Right,
        selected_index: last_idx,
    });

    if drive_count == 0 {
        return Ok(());
    }

    process_popup_input(&mut app, key_event(KeyCode::Home));
    if let Overlay::DriveSelection { selected_index, .. } = &app.overlay {
        assert_eq!(*selected_index, 0);
    }

    Ok(())
}

#[test]
fn test_process_popup_input_drive_selection_unhandled_key() -> io::Result<()> {
    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::DriveSelection {
        side: PanelSide::Left,
        selected_index: 0,
    });

    // Unhandled key should return true but not change anything
    assert!(process_popup_input(&mut app, key_event(KeyCode::F(5))));
    if let Overlay::DriveSelection { selected_index, .. } = &app.overlay {
        assert_eq!(*selected_index, 0);
    }
    assert!(matches!(app.overlay, Overlay::DriveSelection { .. }));

    Ok(())
}
