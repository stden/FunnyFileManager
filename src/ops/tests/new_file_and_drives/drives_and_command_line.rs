//! Purpose: Enter in the drive dialog for the right panel, and typing at the command line.

use super::*;

// --- find_and_select_entry skips dotdot ---

// -----------------------------------------------------------------------
// Drive selection overlay -- Enter navigates the right panel
// -----------------------------------------------------------------------

#[test]
fn test_drive_selection_enter_navigates_right_panel() -> io::Result<()> {
    let mut app = App::new().expect("test setup");
    // Target the right panel.
    app.set_overlay(Overlay::DriveSelection {
        side: crate::models::PanelSide::Right,
        selected_index: 0,
        rows: crate::models::drive_rows(),
    });

    let drives = App::get_all_drives();
    if drives.is_empty() {
        return Ok(());
    }

    // Only proceed when the first drive is a local path that exists.
    let first_local_path = match &drives[0] {
        crate::models::DriveEntry::Local(p) if p.exists() => p.clone(),
        _ => return Ok(()),
    };

    // A drive remembers its current directory, as in Far: when the active
    // (left) panel is already on this drive, the right panel opens the same
    // folder rather than the root.
    let expected = if app.left_panel.path.starts_with(&first_local_path) {
        app.left_panel.path.clone()
    } else {
        first_local_path
    };

    process_popup_input(&mut app, key_event(KeyCode::Enter));

    assert!(
        !matches!(app.overlay, Overlay::DriveSelection { .. }),
        "drive overlay should close after Enter"
    );
    assert_eq!(
        app.right_panel.path, expected,
        "right panel should open the selected drive's current directory"
    );

    Ok(())
}

// -----------------------------------------------------------------------
// Command-line overlay -- Ctrl+Char is ignored (char not appended)
// -----------------------------------------------------------------------

#[test]
fn test_command_line_char_appends_to_buffer() -> io::Result<()> {
    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::CommandLine("he".into()));

    process_popup_input(&mut app, key_event(KeyCode::Char('l')));

    assert_eq!(
        app.get_command_line(),
        Some("hel"),
        "plain char should be appended to command buffer"
    );

    Ok(())
}

#[test]
fn test_command_line_backspace_on_single_char_leaves_empty() -> io::Result<()> {
    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::CommandLine("x".into()));

    process_popup_input(&mut app, key_event(KeyCode::Backspace));

    assert_eq!(
        app.get_command_line(),
        Some(""),
        "backspace on single char should leave Some(\"\"), not None"
    );

    Ok(())
}
