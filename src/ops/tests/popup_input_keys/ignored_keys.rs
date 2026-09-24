//! Purpose: Control, Alt and other keys the prompts ignore, and Backspace on nothing.

use super::*;

// --- Search overlay edge cases ---

/// Ctrl+Char is ignored (char is not appended to the buffer).
#[test]
fn test_new_file_input_ctrl_char_ignored() -> io::Result<()> {
    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::NewFileInput("abc".to_owned()));

    process_popup_input(
        &mut app,
        key_event_with_mods(KeyCode::Char('d'), KeyModifiers::CONTROL),
    );

    assert_eq!(
        if let Overlay::NewFileInput(ref s) = app.overlay {
            s
        } else {
            panic!()
        },
        "abc"
    );

    Ok(())
}

// --- Bookmark overlay edge cases ---

// --- Command line edge cases ---

#[test]
fn test_process_popup_input_command_line_alt_char_ignored() -> io::Result<()> {
    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::CommandLine("cmd".into()));

    // Alt+Char is ignored by the command line but still consumed by it.
    assert!(process_popup_input(
        &mut app,
        key_event_with_mods(KeyCode::Char('x'), KeyModifiers::ALT)
    ));
    if let Overlay::CommandLine(ref s) = app.overlay {
        assert_eq!(s.text(), "cmd");
    } else {
        panic!("Expected CommandLine overlay");
    }

    Ok(())
}

/// Alt+Char is ignored (char is not appended to the buffer).
#[test]
fn test_new_file_input_alt_char_ignored() -> io::Result<()> {
    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::NewFileInput("abc".to_owned()));

    process_popup_input(
        &mut app,
        key_event_with_mods(KeyCode::Char('d'), KeyModifiers::ALT),
    );

    assert_eq!(
        app.get_new_file_input_mut().map(|s| s.as_str()),
        Some("abc")
    );

    Ok(())
}

#[test]
fn test_process_popup_input_command_line_other_key() -> io::Result<()> {
    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::CommandLine("test".into()));

    // F-keys are swallowed: F5 must not copy files behind the open prompt.
    assert!(process_popup_input(&mut app, key_event(KeyCode::F(5))));
    if let Overlay::CommandLine(ref s) = app.overlay {
        assert_eq!(s.text(), "test");
    } else {
        panic!("Expected CommandLine overlay");
    }

    Ok(())
}

/// Unrecognised key in `new_file_input` overlay -- overlay stays open, buffer unchanged.
#[test]
fn test_new_file_input_other_key_ignored() -> io::Result<()> {
    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::NewFileInput("abc".to_owned()));

    process_popup_input(&mut app, key_event(KeyCode::F(5)));

    assert_eq!(
        app.get_new_file_input_mut().map(|s| s.as_str()),
        Some("abc")
    );

    Ok(())
}

/// Backspace on an empty input buffer leaves it empty (no panic).
#[test]
fn test_new_file_input_backspace_empty() -> io::Result<()> {
    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::NewFileInput(String::new()));

    process_popup_input(&mut app, key_event(KeyCode::Backspace));

    assert_eq!(app.get_new_file_input_mut().map(|s| s.as_str()), Some(""));

    Ok(())
}
