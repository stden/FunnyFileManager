//! Purpose: Keys in the new-file prompt: control characters, Esc, Backspace and an existing name.

use super::*;

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
    app.set_overlay(Overlay::NewFileInput("partial".to_owned()));

    process_popup_input(&mut app, key_event(KeyCode::Esc));

    assert!(!matches!(app.overlay, Overlay::NewFileInput(_)));

    Ok(())
}

#[test]
fn test_process_popup_input_new_file_backspace_on_text() -> io::Result<()> {
    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::NewFileInput("abc".to_owned()));

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
    app.set_overlay(Overlay::NewFileInput("test".to_owned()));

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
    app.set_overlay(Overlay::NewFileInput("exists.txt".to_owned()));

    assert!(process_popup_input(&mut app, key_event(KeyCode::Enter)));
    assert!(!matches!(app.overlay, Overlay::NewFileInput(_)));
    assert!(app.status_text().contains("already exists"));

    Ok(())
}
