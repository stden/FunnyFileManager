//! Purpose: The drive list and the drive selection overlay.

use super::*;

#[test]
fn test_app_get_drives() {
    let drives = App::get_drives();
    assert_ne!(drives.len(), 0);
    #[cfg(not(windows))]
    assert_eq!(drives, vec![PathBuf::from("/")]);
}

#[test]
fn test_app_get_all_drives_includes_local() {
    let drives = App::get_all_drives();
    assert_ne!(drives.len(), 0);
}

#[test]
fn test_popup_drive_selection_esc() -> io::Result<()> {
    let mut app = App::new()?;
    app.set_overlay(Overlay::DriveSelection {
        side: PanelSide::Left,
        selected_index: 0,
        rows: funny_file_manager::models::drive_rows(),
    });
    process_popup_input(&mut app, key_event(KeyCode::Esc));
    assert!(!matches!(app.overlay, Overlay::DriveSelection { .. }));
    Ok(())
}

// handlers.rs — drive selection Enter on "/" (local drive)
#[test]
fn test_drive_selection_enter_local() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = setup_app(&temp)?;
    app.set_overlay(Overlay::DriveSelection {
        side: PanelSide::Left,
        selected_index: 0,
        rows: funny_file_manager::models::drive_rows(),
    });
    // Enter selects the first drive
    process_popup_input(&mut app, key_event(KeyCode::Enter));
    assert!(!matches!(app.overlay, Overlay::DriveSelection { .. }));
    Ok(())
}

// handlers.rs — drive selection Up/Down navigation
#[test]
fn test_drive_selection_up_down() -> io::Result<()> {
    let mut app = App::new()?;
    app.set_overlay(Overlay::DriveSelection {
        side: PanelSide::Left,
        selected_index: 0,
        rows: funny_file_manager::models::drive_rows(),
    });
    process_popup_input(&mut app, key_event(KeyCode::Down));
    // Index should advance (or wrap)
    process_popup_input(&mut app, key_event(KeyCode::Up));
    assert!(matches!(app.overlay, Overlay::DriveSelection { .. })); // still open
    Ok(())
}

// DriveEntry display
#[test]
fn test_drive_entry_variants() {
    use funny_file_manager::models::DriveEntry;
    let local = DriveEntry::Local(PathBuf::from("/"));
    let s3 = DriveEntry::S3Profile("prod".to_owned());
    // Just ensure they can be created and debug-printed
    drop(format!("{local:?}"));
    drop(format!("{s3:?}"));
}
