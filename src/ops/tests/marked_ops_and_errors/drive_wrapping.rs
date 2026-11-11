//! Purpose: The drive dialog's cursor wraps both ways, Home, S3 profiles and unhandled keys.

use super::*;

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
        rows: crate::models::drive_rows(),
    });

    let drive_count = App::get_all_drives().len();
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
        rows: crate::models::drive_rows(),
    });

    let drive_count = App::get_all_drives().len();
    if drive_count == 0 {
        return Ok(());
    }

    app.set_overlay(Overlay::DriveSelection {
        side: PanelSide::Left,
        selected_index: drive_count - 1,
        rows: crate::models::drive_rows(),
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
        rows: crate::models::drive_rows(),
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
            rows: crate::models::drive_rows(),
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
        rows: crate::models::drive_rows(),
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
        rows: crate::models::drive_rows(),
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
    let drive_count = App::get_all_drives().len();
    let last_idx = if drive_count == 0 { 0 } else { drive_count - 1 };
    app.set_overlay(Overlay::DriveSelection {
        side: PanelSide::Right,
        selected_index: last_idx,
        rows: crate::models::drive_rows(),
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
        rows: crate::models::drive_rows(),
    });

    // Unhandled key should return true but not change anything
    assert!(process_popup_input(&mut app, key_event(KeyCode::F(5))));
    if let Overlay::DriveSelection { selected_index, .. } = &app.overlay {
        assert_eq!(*selected_index, 0);
    }
    assert!(matches!(app.overlay, Overlay::DriveSelection { .. }));

    Ok(())
}
