//! Purpose: Covers Windows-specific popup and diagnostic behavior.
//!
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use funny_file_manager::handlers::process_popup_input;
use funny_file_manager::models::{App, Overlay};
use std::io;

const fn enter_key() -> KeyEvent {
    KeyEvent::new(KeyCode::Enter, KeyModifiers::empty())
}

#[test]
fn test_command_line_winerrors_behavior() -> io::Result<()> {
    let mut app = App::new()?;
    app.set_overlay(Overlay::CommandLine("winerrors".into()));

    assert!(process_popup_input(&mut app, enter_key()));
    assert!(app.get_command_line().is_none());

    #[cfg(not(windows))]
    assert!(app.status_text().contains("only available on Windows"));

    #[cfg(windows)]
    assert!(
        app.get_viewer().is_some()
            || app.status_text().contains("Opened Windows System Messages")
            || app.status_text().contains("Error:")
    );

    Ok(())
}

#[test]
fn test_command_line_wincheck_behavior() -> io::Result<()> {
    let mut app = App::new()?;
    app.set_overlay(Overlay::CommandLine("wincheck".into()));

    assert!(process_popup_input(&mut app, enter_key()));
    assert!(app.get_command_line().is_none());

    #[cfg(not(windows))]
    assert!(app.status_text().contains("only available on Windows"));

    #[cfg(windows)]
    assert!(
        app.get_viewer().is_some()
            || app.status_text().contains("Opened Windows Performance")
            || app.status_text().contains("Error:")
    );

    Ok(())
}
