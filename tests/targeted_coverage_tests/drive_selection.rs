//! Purpose: The drive selection overlay: Enter, Escape, Home and End.

use super::*;

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
        rows: funny_file_manager::models::drive_rows(),
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
        rows: funny_file_manager::models::drive_rows(),
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
        rows: funny_file_manager::models::drive_rows(),
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
        rows: funny_file_manager::models::drive_rows(),
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
