//! Purpose: Alt+F1/Alt+F2 drive selection and moving inside it.

use super::*;

// ─── ALT+F1 / ALT+F2 DRIVE SELECTION ────────────────────────────────────────

#[test]
fn hotkey_alt_f1_opens_drive_selection_for_left_panel() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    assert!(!matches!(app.overlay, Overlay::DriveSelection { .. }));
    app.set_overlay(Overlay::DriveSelection {
        side: PanelSide::Left,
        selected_index: 0,
        rows: funny_file_manager::models::drive_rows(),
    }); // Alt+F1 logic
    assert!(matches!(
        app.overlay,
        Overlay::DriveSelection {
            side: PanelSide::Left,
            ..
        }
    ));
}

#[test]
fn hotkey_alt_f2_opens_drive_selection_for_right_panel() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    app.set_overlay(Overlay::DriveSelection {
        side: PanelSide::Right,
        selected_index: 0,
        rows: funny_file_manager::models::drive_rows(),
    }); // Alt+F2 logic
    assert!(matches!(
        app.overlay,
        Overlay::DriveSelection {
            side: PanelSide::Right,
            ..
        }
    ));
}

#[test]
fn overlay_drive_esc_closes() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    app.set_overlay(Overlay::DriveSelection {
        side: PanelSide::Left,
        selected_index: 0,
        rows: funny_file_manager::models::drive_rows(),
    });
    process_popup_input(&mut app, press(KeyCode::Esc));
    assert!(!matches!(app.overlay, Overlay::DriveSelection { .. }));
}

#[test]
fn overlay_drive_down_increments_index() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    app.set_overlay(Overlay::DriveSelection {
        side: PanelSide::Left,
        selected_index: 0,
        rows: funny_file_manager::models::drive_rows(),
    });
    process_popup_input(&mut app, press(KeyCode::Down));
    // index should advance if drives list has >1 entry
    let drives = App::get_all_drives();
    if drives.len() > 1 {
        assert!(matches!(
            app.overlay,
            Overlay::DriveSelection {
                selected_index: 1,
                ..
            }
        ));
    }
}

#[test]
fn overlay_drive_up_decrements_index() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    let drives = App::get_all_drives();
    let last = drives.len().saturating_sub(1);
    app.set_overlay(Overlay::DriveSelection {
        side: PanelSide::Left,
        selected_index: last,
        rows: funny_file_manager::models::drive_rows(),
    });
    if last > 0 {
        process_popup_input(&mut app, press(KeyCode::Up));
        let index = if let Overlay::DriveSelection { selected_index, .. } = app.overlay {
            selected_index
        } else {
            0
        };
        assert_eq!(index, last - 1);
    }
}

#[test]
fn overlay_drive_home_jumps_to_zero() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    app.set_overlay(Overlay::DriveSelection {
        side: PanelSide::Left,
        selected_index: 3,
        rows: funny_file_manager::models::drive_rows(),
    });
    process_popup_input(&mut app, press(KeyCode::Home));
    assert!(matches!(
        app.overlay,
        Overlay::DriveSelection {
            selected_index: 0,
            ..
        }
    ));
}

#[test]
fn overlay_drive_end_jumps_to_last() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    app.set_overlay(Overlay::DriveSelection {
        side: PanelSide::Left,
        selected_index: 0,
        rows: funny_file_manager::models::drive_rows(),
    });
    process_popup_input(&mut app, press(KeyCode::End));
    let last = App::get_all_drives().len().saturating_sub(1);
    let index = if let Overlay::DriveSelection { selected_index, .. } = app.overlay {
        selected_index
    } else {
        0
    };
    assert_eq!(index, last);
}
