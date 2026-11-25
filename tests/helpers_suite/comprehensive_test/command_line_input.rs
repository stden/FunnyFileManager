//! Purpose: Command-line editing, help, quit and unknown commands.

use super::*;

#[test]
fn test_command_line_help() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    let (mut app, mut terminal) = setup_app(&temp)?;
    app.set_overlay(Overlay::CommandLine("help".into()));

    let events = vec![
        key(KeyCode::Enter), // opens help
        key(KeyCode::Esc),   // close help
        key(KeyCode::F(10)),
        // F10 now asks before quitting; Enter answers Yes.
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)
}

#[test]
fn test_command_line_quit() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    let (mut app, mut terminal) = setup_app(&temp)?;
    app.set_overlay(Overlay::CommandLine("quit".into()));

    let events = vec![key(KeyCode::Enter)]; // quit command

    run_events(&mut terminal, app, events)
}

#[test]
fn test_command_line_exit() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    let (mut app, mut terminal) = setup_app(&temp)?;
    app.set_overlay(Overlay::CommandLine("exit".into()));

    let events = vec![key(KeyCode::Enter)]; // exit command

    run_events(&mut terminal, app, events)
}

#[test]
fn test_command_line_unknown() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    let (mut app, mut terminal) = setup_app(&temp)?;
    app.set_overlay(Overlay::CommandLine("foobar".into()));

    let events = vec![
        key(KeyCode::Enter),
        key(KeyCode::F(10)),
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)
}

#[test]
fn test_command_line_empty_enter() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    let (mut app, mut terminal) = setup_app(&temp)?;
    app.set_overlay(Overlay::CommandLine("".into()));

    let events = vec![
        key(KeyCode::Enter),
        key(KeyCode::F(10)),
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)
}

#[test]
fn test_command_line_escape() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    let (mut app, mut terminal) = setup_app(&temp)?;
    app.set_overlay(Overlay::CommandLine("some text".into()));

    let events = vec![key(KeyCode::Esc), key(KeyCode::F(10)), key(KeyCode::Enter)];

    run_events(&mut terminal, app, events)
}

#[test]
fn test_command_line_backspace() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    let (mut app, mut terminal) = setup_app(&temp)?;
    app.set_overlay(Overlay::CommandLine("ab".into()));

    let events = vec![
        key(KeyCode::Backspace), // "a"
        key(KeyCode::Backspace), // ""
        key(KeyCode::Esc),
        key(KeyCode::F(10)),
        // F10 now asks before quitting; Enter answers Yes.
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)
}

#[test]
fn test_command_line_type_chars() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    let (mut app, mut terminal) = setup_app(&temp)?;
    app.set_overlay(Overlay::CommandLine(String::new().into()));

    let events = vec![
        key(KeyCode::Char('p')),
        key(KeyCode::Char('w')),
        key(KeyCode::Char('d')),
        key(KeyCode::Enter),
        key(KeyCode::F(10)),
        // F10 now asks before quitting; Enter answers Yes.
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)
}
