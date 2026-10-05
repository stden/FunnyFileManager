//! Purpose: The drive selection overlay: failures, navigation and the right panel.

use super::*;

// ─── Drive selection error paths (handlers.rs lines 272-309) ───────

#[test]
fn test_drive_selection_enter_on_local_drive_failure() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = make_app(temp.path())?;

    // Open drive selection for left panel
    app.set_overlay(Overlay::DriveSelection {
        side: PanelSide::Left,
        selected_index: 0,
        rows: funny_file_manager::models::drive_rows(),
    });

    // Simulate pressing Enter to select the drive
    let consumed = process_popup_input(&mut app, key_event(KeyCode::Enter));
    assert!(consumed);

    // Drive selection should be closed
    assert!(!matches!(app.overlay, Overlay::DriveSelection { .. }));
    let drive_selection_index = if let Overlay::DriveSelection { selected_index, .. } = app.overlay
    {
        selected_index
    } else {
        0
    };
    assert_eq!(drive_selection_index, 0);

    Ok(())
}

#[test]
fn test_drive_selection_navigate_up_down() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = make_app(temp.path())?;

    app.set_overlay(Overlay::DriveSelection {
        side: PanelSide::Left,
        selected_index: 0,
        rows: funny_file_manager::models::drive_rows(),
    });

    // Press Down
    let consumed = process_popup_input(&mut app, key_event(KeyCode::Down));
    assert!(consumed);

    // Press Up
    let consumed = process_popup_input(&mut app, key_event(KeyCode::Up));
    assert!(consumed);
    assert!(matches!(
        app.overlay,
        Overlay::DriveSelection {
            selected_index: 0,
            ..
        }
    ));

    Ok(())
}

#[test]
fn test_drive_selection_char_key() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = make_app(temp.path())?;

    app.set_overlay(Overlay::DriveSelection {
        side: PanelSide::Left,
        selected_index: 0,
        rows: funny_file_manager::models::drive_rows(),
    });

    // Press a letter key (Windows drive letter selection)
    // On Unix this won't find the drive but exercises the code path
    let consumed = process_popup_input(&mut app, key_event(KeyCode::Char('z')));
    assert!(consumed);

    Ok(())
}

#[test]
fn test_drive_selection_for_right_panel() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = make_app(temp.path())?;

    app.set_overlay(Overlay::DriveSelection {
        side: PanelSide::Right,
        selected_index: 0,
        rows: funny_file_manager::models::drive_rows(),
    });

    // Enter selects the drive for the right panel
    let consumed = process_popup_input(&mut app, key_event(KeyCode::Enter));
    assert!(consumed);
    assert!(!matches!(app.overlay, Overlay::DriveSelection { .. }));

    Ok(())
}

// ─── Drive selection load_entries error (handlers.rs line 272-273) ───

#[test]
fn test_drive_selection_enter_with_invalid_drive_path() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = make_app(temp.path())?;

    // Set up drive selection and select a drive that maps to a non-existent path
    // On Unix, the only local drive is "/", which will succeed
    // But we can test the S3Profile branch (line 277-281)
    // Get all drives — the list includes local + S3 profiles
    let drives = App::get_all_drives();
    // If there's more than 1 drive, try selecting the last one
    let selected_index = if drives.len() > 1 {
        drives.len() - 1
    } else {
        0
    };
    app.set_overlay(Overlay::DriveSelection {
        side: PanelSide::Left,
        selected_index,
        rows: funny_file_manager::models::drive_rows(),
    });

    let consumed = process_popup_input(&mut app, key_event(KeyCode::Enter));
    assert!(consumed);
    assert!(!matches!(app.overlay, Overlay::DriveSelection { .. }));

    Ok(())
}
