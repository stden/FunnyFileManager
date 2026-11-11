//! Purpose: Each overlay's keys reach it: drives, delete confirm, command line, new file.

use super::*;

#[test]
fn test_process_popup_input_drive_selection() -> io::Result<()> {
    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::DriveSelection {
        side: PanelSide::Left,
        selected_index: 0,
        rows: crate::models::drive_rows(),
    });

    process_popup_input(&mut app, KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    process_popup_input(&mut app, KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
    process_popup_input(&mut app, KeyEvent::new(KeyCode::End, KeyModifiers::NONE));
    process_popup_input(&mut app, KeyEvent::new(KeyCode::Home, KeyModifiers::NONE));
    if let Overlay::DriveSelection { selected_index, .. } = &app.overlay {
        assert_eq!(*selected_index, 0);
    }

    process_popup_input(&mut app, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert!(!matches!(app.overlay, Overlay::DriveSelection { .. }));

    app.set_overlay(Overlay::DriveSelection {
        side: PanelSide::Left,
        selected_index: 0,
        rows: crate::models::drive_rows(),
    });
    process_popup_input(&mut app, key_event(KeyCode::Enter));
    assert!(!matches!(app.overlay, Overlay::DriveSelection { .. }));

    Ok(())
}

#[test]
fn test_process_popup_input_delete_confirm() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let file_path = temp_dir.path().join("to_delete.txt");
    File::create(&file_path).expect("test setup");

    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::DeleteConfirm(vec![file_path.clone()]));

    process_popup_input(
        &mut app,
        KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE),
    );
    assert!(!matches!(app.overlay, Overlay::DeleteConfirm(_)));
    assert!(file_path.exists());

    app.set_overlay(Overlay::DeleteConfirm(vec![file_path.clone()]));
    process_popup_input(
        &mut app,
        KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE),
    );
    assert!(!matches!(app.overlay, Overlay::DeleteConfirm(_)));
    assert!(!file_path.exists());

    Ok(())
}

#[test]
fn test_process_popup_input_command_line_trigger() -> io::Result<()> {
    let mut app = App::new().expect("test setup");

    // Simulating the ':' key being pressed in the main loop logic
    app.set_overlay(Overlay::CommandLine(CommandLine::default()));

    // Now test interaction with the command line overlay
    process_popup_input(
        &mut app,
        KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE),
    );
    process_popup_input(
        &mut app,
        KeyEvent::new(KeyCode::Char('w'), KeyModifiers::NONE),
    );
    process_popup_input(
        &mut app,
        KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE),
    );

    if let Overlay::CommandLine(cmd) = &app.overlay {
        assert_eq!(cmd.text(), "pwd");
    }

    // Enter should execute and close
    process_popup_input(&mut app, key_event(KeyCode::Enter));
    assert!(!matches!(app.overlay, Overlay::CommandLine(_)));

    Ok(())
}

#[test]
fn test_process_popup_input_new_file_input() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let mut app = App::new().expect("test setup");
    app.left_panel.path = temp_dir.path().to_path_buf();
    app.set_overlay(Overlay::NewFileInput(String::new().into()));

    for character in "new.txt".chars() {
        process_popup_input(&mut app, key_event(KeyCode::Char(character)));
    }

    if let Overlay::NewFileInput(input) = &app.overlay {
        assert_eq!(input.text(), "new.txt");
    }

    process_popup_input(&mut app, key_event(KeyCode::Backspace));
    if let Overlay::NewFileInput(input) = &app.overlay {
        assert_eq!(input.text(), "new.tx");
    }
    process_popup_input(&mut app, key_event(KeyCode::Char('t')));

    process_popup_input(&mut app, key_event(KeyCode::Enter));
    assert!(!matches!(app.overlay, Overlay::NewFileInput(_)));
    assert!(temp_dir.path().join("new.txt").exists());

    // Test empty filename
    app.set_overlay(Overlay::NewFileInput("  ".to_owned().into()));
    process_popup_input(&mut app, key_event(KeyCode::Enter));
    assert!(!matches!(app.overlay, Overlay::NewFileInput(_)));
    assert!(app.status_text().contains("cannot be empty"));

    // Test existing file
    app.set_overlay(Overlay::NewFileInput("new.txt".to_owned().into()));
    process_popup_input(&mut app, key_event(KeyCode::Enter));
    assert!(!matches!(app.overlay, Overlay::NewFileInput(_)));
    assert!(app.status_text().contains("already exists"));

    Ok(())
}

#[test]
fn test_execute_command_pwd_quit_exit() -> io::Result<()> {
    let mut app = App::new().expect("test setup");

    execute_command(&mut app, "pwd").expect("test setup");
    assert_ne!(app.status_text().len(), 0);

    execute_command(&mut app, "quit").expect("test setup");
    assert!(!app.lifecycle.is_running());

    app.lifecycle = Lifecycle::Running;
    execute_command(&mut app, "exit").expect("test setup");
    assert!(!app.lifecycle.is_running());

    Ok(())
}
