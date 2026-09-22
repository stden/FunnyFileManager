//! Unit tests: drive selection (split out to keep files under 500 lines).
#![allow(unused_imports)]

use super::support::*;
use crate::popup::process_popup_input;

#[cfg(feature = "test-helpers")]
use crate::editor::{handle_edit_impl, handle_edit_test_mode, TestEditorBehavior};

use crate::models::{App, Overlay, Panel, PanelSide};

#[cfg(feature = "test-helpers")]
use crate::WritableTestBackend;

use crossterm::event::{KeyCode, KeyModifiers};

use std::fs::{self, File};

use std::io::{self, Write};

use std::path::PathBuf;

use tempfile::TempDir;

/// End key jumps to last index.
#[test]
fn test_drive_selection_end() -> io::Result<()> {
    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::DriveSelection {
        side: PanelSide::Left,
        selected_index: 0,
        rows: crate::models::drive_rows(),
    });

    let drive_count = crate::models::App::get_all_drives().len();
    if drive_count == 0 {
        return Ok(());
    }

    process_popup_input(&mut app, key_event(KeyCode::End));
    if let Overlay::DriveSelection { selected_index, .. } = &app.overlay {
        assert_eq!(*selected_index, drive_count - 1);
    }

    Ok(())
}

/// Enter on the first drive entry navigates the right panel (target = Right).
#[test]
fn test_drive_selection_enter_right_panel() -> io::Result<()> {
    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::DriveSelection {
        side: PanelSide::Right,
        selected_index: 0,
        rows: crate::models::drive_rows(),
    });

    let drives = crate::models::App::get_all_drives();
    if drives.is_empty() {
        return Ok(());
    }

    // Only proceed when the first drive is a Local drive that exists.
    if let crate::models::DriveEntry::Local(ref path) = drives[0] {
        if !path.exists() {
            return Ok(());
        }
    }

    process_popup_input(&mut app, key_event(KeyCode::Enter));
    // After Enter the overlay should be cleared.
    assert!(!matches!(app.overlay, Overlay::DriveSelection { .. }));

    Ok(())
}

/// Char key for a drive letter that does NOT exist on the filesystem.
/// The overlay should remain open and no navigation should occur.
#[test]
fn test_drive_selection_char_nonexistent_drive() -> io::Result<()> {
    // Pick a drive letter that almost certainly does not exist.
    // On both Windows and Unix 'Q' is unlikely to be a real drive root.
    let drive_letter = 'Q';
    let probe = PathBuf::from(format!("{drive_letter}:\\"));
    if probe.exists() {
        // If somehow Q:\ exists, skip the test.
        return Ok(());
    }

    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::DriveSelection {
        side: PanelSide::Left,
        selected_index: 0,
        rows: crate::models::drive_rows(),
    });
    let original_path = app.left_panel.path.clone();

    process_popup_input(
        &mut app,
        key_event(KeyCode::Char(drive_letter.to_ascii_lowercase())),
    );

    // Overlay stays open because the drive does not exist.
    assert!(matches!(app.overlay, Overlay::DriveSelection { .. }));
    // Panel path unchanged.
    assert_eq!(app.left_panel.path, original_path);

    Ok(())
}

/// Non-alphabetic, non-navigation key in drive selection – does nothing.
#[test]
fn test_drive_selection_other_key_ignored() -> io::Result<()> {
    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::DriveSelection {
        side: PanelSide::Left,
        selected_index: 0,
        rows: crate::models::drive_rows(),
    });

    process_popup_input(&mut app, key_event(KeyCode::F(5)));

    // Overlay still active.
    assert!(matches!(app.overlay, Overlay::DriveSelection { .. }));

    Ok(())
}

/// Enter with an `S3Profile` drive shows the expected status message.
#[test]
fn test_drive_selection_enter_s3_profile() -> io::Result<()> {
    use crate::models::DriveEntry;

    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::DriveSelection {
        side: PanelSide::Left,
        selected_index: 0,
        rows: crate::models::drive_rows(),
    });

    // Find an S3 profile entry in get_all_drives(); if none configured, skip.
    let drives = crate::models::App::get_all_drives();
    let s3_index = drives
        .iter()
        .position(|d| matches!(d, DriveEntry::S3Profile(_)));

    let Some(idx) = s3_index else {
        return Ok(()); // No S3 profiles configured – skip.
    };

    if let Overlay::DriveSelection {
        ref mut selected_index,
        ..
    } = app.overlay
    {
        *selected_index = idx;
    }

    process_popup_input(&mut app, key_event(KeyCode::Enter));

    assert!(!matches!(app.overlay, Overlay::DriveSelection { .. }));
    assert!(app.status_text().contains("S3 profile") || app.status_text().contains("coming soon"));

    Ok(())
}

// -----------------------------------------------------------------------
// Delete confirmation overlay – additional branch coverage
// -----------------------------------------------------------------------

// --- Delete confirm overlay keys ---

#[test]
fn test_process_popup_input_delete_confirm_enter() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let file_path = temp_dir.path().join("del.txt");
    fs::write(&file_path, b"data").expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.set_overlay(Overlay::DeleteConfirm(vec![file_path.clone()]));

    // Enter key should confirm delete
    assert!(process_popup_input(&mut app, key_event(KeyCode::Enter)));
    assert!(!file_path.exists());
    assert!(!matches!(app.overlay, Overlay::DeleteConfirm(_)));

    Ok(())
}

/// 'Y' (uppercase) confirms deletion.
#[test]
fn test_delete_confirm_uppercase_y() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let file_path = temp_dir.path().join("del_Y.txt");
    File::create(&file_path).expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.set_overlay(Overlay::DeleteConfirm(vec![file_path.clone()]));

    process_popup_input(&mut app, key_event(KeyCode::Char('Y')));

    assert!(!matches!(app.overlay, Overlay::DeleteConfirm(_)));
    assert!(!file_path.exists());

    Ok(())
}

/// Enter key confirms deletion.
#[test]
fn test_delete_confirm_enter() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let file_path = temp_dir.path().join("del_enter.txt");
    File::create(&file_path).expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.set_overlay(Overlay::DeleteConfirm(vec![file_path.clone()]));

    process_popup_input(&mut app, key_event(KeyCode::Enter));

    assert!(!matches!(app.overlay, Overlay::DeleteConfirm(_)));
    assert!(!file_path.exists());

    Ok(())
}

#[test]
fn test_process_popup_input_delete_confirm_uppercase_n() -> io::Result<()> {
    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::DeleteConfirm(vec![PathBuf::from("/tmp/x")]));

    assert!(process_popup_input(&mut app, key_event(KeyCode::Char('N'))));
    assert!(!matches!(app.overlay, Overlay::DeleteConfirm(_)));
    assert!(app.status_text().contains("cancelled"));

    Ok(())
}

/// 'N' (uppercase) cancels deletion.
#[test]
fn test_delete_confirm_uppercase_n() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let file_path = temp_dir.path().join("del_N.txt");
    File::create(&file_path).expect("test setup");

    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::DeleteConfirm(vec![file_path.clone()]));

    process_popup_input(&mut app, key_event(KeyCode::Char('N')));

    assert!(!matches!(app.overlay, Overlay::DeleteConfirm(_)));
    assert!(file_path.exists());
    assert!(app.status_text().contains("cancelled"));

    Ok(())
}

#[test]
fn test_process_popup_input_delete_confirm_other_key() -> io::Result<()> {
    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::DeleteConfirm(vec![PathBuf::from("/tmp/x")]));

    // Other key should be consumed but not act
    assert!(process_popup_input(&mut app, key_event(KeyCode::Down)));
    assert!(matches!(app.overlay, Overlay::DeleteConfirm(_)));

    Ok(())
}

/// Esc key cancels deletion.
#[test]
fn test_delete_confirm_esc_cancels() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let file_path = temp_dir.path().join("del_esc.txt");
    File::create(&file_path).expect("test setup");

    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::DeleteConfirm(vec![file_path.clone()]));

    process_popup_input(&mut app, key_event(KeyCode::Esc));

    assert!(!matches!(app.overlay, Overlay::DeleteConfirm(_)));
    assert!(file_path.exists());
    assert!(app.status_text().contains("cancelled"));

    Ok(())
}

/// Any other key leaves `delete_confirm` unchanged (overlay stays open).
#[test]
fn test_delete_confirm_other_key_stays_open() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let file_path = temp_dir.path().join("del_other.txt");
    File::create(&file_path).expect("test setup");

    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::DeleteConfirm(vec![file_path.clone()]));

    // A key that is not y/Y/n/N/Enter/Esc.
    process_popup_input(&mut app, key_event(KeyCode::Char('x')));

    // delete_confirm must still be Some – the overlay stays open.
    assert!(matches!(app.overlay, Overlay::DeleteConfirm(_)));
    // File must not be deleted.
    assert!(file_path.exists());

    Ok(())
}

// --- New file input edge cases ---

// -----------------------------------------------------------------------
// New file input overlay – additional branch coverage
// -----------------------------------------------------------------------

#[test]
fn test_process_popup_input_new_file_ctrl_char_ignored() -> io::Result<()> {
    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::NewFileInput(String::new()));

    // Ctrl+char should not add to input
    assert!(process_popup_input(
        &mut app,
        key_event_with_mods(KeyCode::Char('a'), KeyModifiers::CONTROL)
    ));
    assert_eq!(
        if let Overlay::NewFileInput(ref s) = app.overlay {
            s
        } else {
            panic!()
        },
        ""
    );

    // Alt+char should not add to input
    assert!(process_popup_input(
        &mut app,
        key_event_with_mods(KeyCode::Char('b'), KeyModifiers::ALT)
    ));
    assert_eq!(
        if let Overlay::NewFileInput(ref s) = app.overlay {
            s
        } else {
            panic!()
        },
        ""
    );

    Ok(())
}

/// Esc clears `new_file_input`.
#[test]
fn test_new_file_input_esc() -> io::Result<()> {
    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::NewFileInput("partial".to_string()));

    process_popup_input(&mut app, key_event(KeyCode::Esc));

    assert!(!matches!(app.overlay, Overlay::NewFileInput(_)));

    Ok(())
}

#[test]
fn test_process_popup_input_new_file_backspace_on_text() -> io::Result<()> {
    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::NewFileInput("abc".to_string()));

    assert!(process_popup_input(&mut app, key_event(KeyCode::Backspace)));
    assert_eq!(
        if let Overlay::NewFileInput(ref s) = app.overlay {
            s
        } else {
            panic!()
        },
        "ab"
    );

    Ok(())
}

#[test]
fn test_process_popup_input_new_file_other_key() -> io::Result<()> {
    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::NewFileInput("test".to_string()));

    // F5 key - should be consumed but not change input
    assert!(process_popup_input(&mut app, key_event(KeyCode::F(5))));
    assert_eq!(
        if let Overlay::NewFileInput(ref s) = app.overlay {
            s
        } else {
            panic!()
        },
        "test"
    );

    Ok(())
}

#[test]
fn test_process_popup_input_new_file_already_exists() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    fs::write(temp_dir.path().join("exists.txt"), b"data").expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.set_overlay(Overlay::NewFileInput("exists.txt".to_string()));

    assert!(process_popup_input(&mut app, key_event(KeyCode::Enter)));
    assert!(!matches!(app.overlay, Overlay::NewFileInput(_)));
    assert!(app.status_text().contains("already exists"));

    Ok(())
}
