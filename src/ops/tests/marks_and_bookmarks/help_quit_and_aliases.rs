//! Purpose: help, quit and exit, the Windows tool aliases, and an unknown command.

use super::*;

#[test]
fn test_execute_command_help() -> io::Result<()> {
    let mut app = App::new().expect("test setup");

    execute_command(&mut app, "help").expect("test setup");
    assert!(matches!(app.overlay, Overlay::Help));

    Ok(())
}

#[test]
fn test_execute_command_help_direct() -> io::Result<()> {
    let mut app = App::new().expect("test setup");
    assert!(!matches!(app.overlay, Overlay::Help));

    execute_command(&mut app, "help").expect("help command");

    assert!(
        matches!(app.overlay, Overlay::Help),
        "help command should set overlay to Help"
    );

    Ok(())
}

#[test]
fn test_execute_command_exit() -> io::Result<()> {
    let mut app = App::new().expect("test setup");

    execute_command(&mut app, "exit").expect("test setup");
    assert!(!app.lifecycle.is_running());

    Ok(())
}

#[test]
fn test_execute_command_quit_direct() -> io::Result<()> {
    let mut app = App::new().expect("test setup");
    assert!(app.lifecycle.is_running());

    execute_command(&mut app, "quit").expect("quit command");

    assert!(
        !app.lifecycle.is_running(),
        "quit command should set should_quit = true"
    );

    Ok(())
}

#[test]
#[cfg(not(windows))]
fn test_execute_command_windows_system_aliases() -> io::Result<()> {
    for alias in ["winmsg", "winmsgs", "winerr", "winerrors"] {
        let mut app = App::new().expect("test setup");

        execute_command(&mut app, alias)?;

        assert_eq!(
            app.status_text(),
            "Windows system checker is only available on Windows"
        );
        assert!(app.get_viewer().is_none());
    }

    Ok(())
}

#[test]
#[cfg(not(windows))]
fn test_execute_command_windows_performance_aliases() -> io::Result<()> {
    for alias in ["winperf", "wincheck"] {
        let mut app = App::new().expect("test setup");

        execute_command(&mut app, alias)?;

        assert_eq!(
            app.status_text(),
            "Windows performance checker is only available on Windows"
        );
        assert!(app.get_viewer().is_none());
    }

    Ok(())
}

#[test]
fn test_execute_command_unknown_returns_error() -> io::Result<()> {
    let mut app = App::new().expect("test setup");

    // Unknown commands are dispatched to the shell; the shell process
    // runs successfully (Ok) even though the command is not found.
    let result = execute_command(&mut app, "frobnicator");
    assert!(result.is_ok());
    // The status_message should capture the shell's error output.
    assert_ne!(app.status_text().len(), 0);

    Ok(())
}
