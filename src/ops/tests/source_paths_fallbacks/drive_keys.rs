//! Purpose: The drive dialog's Up, Down and Enter on a local drive or an S3 profile.

use super::*;

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
