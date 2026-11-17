//! Purpose: Command-line sort and mark commands.

use super::*;

#[test]
fn test_command_line_sort_name() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    let (mut app, mut terminal) = setup_app(&temp)?;
    app.set_overlay(Overlay::CommandLine("sort name".into()));

    let events = vec![
        key(KeyCode::Enter),
        key(KeyCode::F(10)),
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)
}

#[test]
fn test_command_line_sort_ext() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    let (mut app, mut terminal) = setup_app(&temp)?;
    app.set_overlay(Overlay::CommandLine("sort ext".into()));

    let events = vec![
        key(KeyCode::Enter),
        key(KeyCode::F(10)),
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)
}

#[test]
fn test_command_line_sort_extension() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    let (mut app, mut terminal) = setup_app(&temp)?;
    app.set_overlay(Overlay::CommandLine("sort extension".into()));

    let events = vec![
        key(KeyCode::Enter),
        key(KeyCode::F(10)),
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)
}

#[test]
fn test_command_line_sort_size() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    let (mut app, mut terminal) = setup_app(&temp)?;
    app.set_overlay(Overlay::CommandLine("sort size".into()));

    let events = vec![
        key(KeyCode::Enter),
        key(KeyCode::F(10)),
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)
}

#[test]
fn test_command_line_sort_invalid() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    let (mut app, mut terminal) = setup_app(&temp)?;
    app.set_overlay(Overlay::CommandLine("sort blah".into()));

    let events = vec![
        key(KeyCode::Enter),
        key(KeyCode::F(10)),
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)
}

#[test]
fn test_command_line_mark_all() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("a.txt"), b"").unwrap();
    fs::write(temp.path().join("b.txt"), b"").unwrap();

    let (mut app, mut terminal) = setup_app(&temp)?;
    app.set_overlay(Overlay::CommandLine("mark all".into()));

    let events = vec![
        key(KeyCode::Enter),
        key(KeyCode::F(10)),
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)
}

#[test]
fn test_command_line_unmark() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    let (mut app, mut terminal) = setup_app(&temp)?;
    app.left_panel.marked.insert(FileName::from("a.txt"));
    app.set_overlay(Overlay::CommandLine("unmark".into()));

    let events = vec![
        key(KeyCode::Enter),
        key(KeyCode::F(10)),
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)
}

#[test]
fn test_command_mark_invalid_sub() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    let (mut app, mut terminal) = setup_app(&temp)?;
    app.set_overlay(Overlay::CommandLine("mark blah".into()));

    let events = vec![
        key(KeyCode::Enter),
        key(KeyCode::F(10)),
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)
}
