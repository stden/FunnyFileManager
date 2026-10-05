//! Purpose: Typing, Enter and Backspace at the command-line prompt.

use super::*;

#[test]
fn test_process_popup_input_command_line() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    app.set_overlay(Overlay::CommandLine(CommandLine::default()));
    assert!(process_popup_input(&mut app, key_event(KeyCode::Char('p'))));
    assert_eq!(app.get_command_line(), Some("p"));

    // Alt+Char is not typed into the buffer, but the modal prompt still
    // consumes it — it must never reach the panel handler.
    assert!(process_popup_input(
        &mut app,
        key_event_with_mods(KeyCode::Char('x'), KeyModifiers::ALT)
    ));
    assert_eq!(app.get_command_line(), Some("p"));

    assert!(process_popup_input(&mut app, key_event(KeyCode::Backspace)));
    assert_eq!(app.get_command_line(), Some(""));

    assert!(process_popup_input(&mut app, key_event(KeyCode::Enter)));
    assert!(!matches!(app.overlay, Overlay::CommandLine(_)));

    app.set_overlay(Overlay::CommandLine("pwd".into()));
    assert!(process_popup_input(&mut app, key_event(KeyCode::Enter)));
    assert_eq!(app.status_text(), temp_dir.path().display().to_string());

    app.set_overlay(Overlay::CommandLine("unknown".into()));
    assert!(process_popup_input(&mut app, key_event(KeyCode::Enter)));
    // "unknown" is run as a shell command; stderr produces an error message
    // that is captured as "Err: ..." in status_message.
    assert!(!app.status_text().is_empty());

    app.set_overlay(Overlay::CommandLine("quit".into()));
    assert!(process_popup_input(&mut app, key_event(KeyCode::Enter)));
    assert!(!app.lifecycle.is_running());

    app.lifecycle = Lifecycle::Running;
    app.set_overlay(Overlay::CommandLine("pwd".into()));
    assert!(process_popup_input(&mut app, key_event(KeyCode::Esc)));
    assert!(!matches!(app.overlay, Overlay::CommandLine(_)));

    Ok(())
}

#[test]
fn test_process_popup_input_command_line_enter_empty() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.set_overlay(Overlay::CommandLine("   ".into()));
    app.set_status(Status::CommandOutput("status".to_owned()));

    assert!(process_popup_input(&mut app, key_event(KeyCode::Enter)));
    assert!(!matches!(app.overlay, Overlay::CommandLine(_)));
    assert_eq!(app.status_text(), "status");

    Ok(())
}

#[test]
fn test_process_popup_input_command_line_backspace_empty() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.set_overlay(Overlay::CommandLine(CommandLine::default()));

    assert!(process_popup_input(&mut app, key_event(KeyCode::Backspace)));
    assert_eq!(app.get_command_line(), Some(""));

    Ok(())
}

#[test]
fn test_process_popup_input_command_line_ignores_control_char() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.set_overlay(Overlay::CommandLine("a".into()));

    // Ctrl+Char is not typed into the buffer, but the modal prompt consumes it.
    assert!(process_popup_input(
        &mut app,
        key_event_with_mods(KeyCode::Char('b'), KeyModifiers::CONTROL)
    ));
    assert_eq!(app.get_command_line(), Some("a"));

    Ok(())
}
