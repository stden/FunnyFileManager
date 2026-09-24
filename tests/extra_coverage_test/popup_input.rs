//! Purpose: Popup input for drive selection, new-file input and delete confirmation.

use super::*;

#[test]
fn test_process_popup_input_drives() -> io::Result<()> {
    let mut app = App::new()?;
    app.set_overlay(Overlay::DriveSelection {
        side: PanelSide::Left,
        selected_index: 0,
        rows: funny_file_manager::models::drive_rows(),
    });
    let drives = App::get_all_drives();
    let drive_count = drives.len();

    for (code, expected) in [
        (KeyCode::Down, usize::from(drive_count > 1)),
        (KeyCode::Up, 0),
        (KeyCode::Up, drive_count.saturating_sub(1)),
        (KeyCode::End, drive_count.saturating_sub(1)),
        (KeyCode::Home, 0),
    ] {
        process_popup_input(&mut app, key_event(code));
        assert!(matches!(app.overlay,
            Overlay::DriveSelection { selected_index, .. } if selected_index == expected));
    }

    // Test Esc
    process_popup_input(&mut app, key_event(KeyCode::Esc));
    assert!(!matches!(app.overlay, Overlay::DriveSelection { .. }));

    Ok(())
}

#[test]
fn test_process_popup_input_new_file_input() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.set_overlay(Overlay::NewFileInput("newfile.txt".to_owned()));

    // Test typing
    process_popup_input(&mut app, key_event(KeyCode::Char('2')));
    assert_eq!(app.get_new_file_input(), Some("newfile.txt2"));

    // Test backspace
    process_popup_input(&mut app, key_event(KeyCode::Backspace));
    assert_eq!(app.get_new_file_input(), Some("newfile.txt"));

    // Test Enter to create file
    process_popup_input(&mut app, key_event(KeyCode::Enter));
    assert!(app.get_new_file_input().is_none());
    assert!(temp_dir.path().join("newfile.txt").exists());
    assert!(app.status_text().contains("Created file"));

    // Test create already existing file
    app.set_overlay(Overlay::NewFileInput("newfile.txt".to_owned()));
    process_popup_input(&mut app, key_event(KeyCode::Enter));
    assert!(app.status_text().contains("already exists"));

    // Test Esc
    app.set_overlay(Overlay::NewFileInput("anything".to_owned()));
    process_popup_input(&mut app, key_event(KeyCode::Esc));
    assert!(app.get_new_file_input().is_none());

    Ok(())
}

#[test]
fn test_handle_delete_confirm_parent_dir() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;

    // Select ".." entry
    let dotdot_index = app
        .left_panel
        .entries
        .iter()
        .position(|e| e.name == "..")
        .unwrap();
    app.left_panel.select(Some(dotdot_index));

    handle_delete_confirm(&mut app)?;
    assert!(app.get_delete_confirm().is_none());

    Ok(())
}

#[test]
fn test_handle_delete_cancel() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("keep_me.txt");
    File::create(&file_path)?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.left_panel.select(Some(1)); // Select the file

    handle_delete_confirm(&mut app)?;
    assert!(app.get_delete_confirm().is_some());

    // Cancel with 'n'
    process_popup_input(&mut app, key_event(KeyCode::Char('n')));
    assert!(app.get_delete_confirm().is_none());
    assert!(file_path.exists());
    assert!(app.status_text().contains("cancelled"));

    Ok(())
}
