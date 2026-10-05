//! Purpose: Built-in commands: pwd, sort, mark, help, quit and empty input.

use super::*;

#[test]
fn test_command_line_pwd() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel.path = temp.path().to_path_buf();

    app.set_overlay(Overlay::CommandLine("pwd".into()));
    process_popup_input(&mut app, key(KeyCode::Enter));
    assert!(app
        .status_text()
        .contains(&temp.path().display().to_string()));

    Ok(())
}

#[test]
fn test_command_line_sort() -> io::Result<()> {
    let temp = TempDir::new()?;
    File::create(temp.path().join("a.txt"))?.write_all(b"x")?;
    File::create(temp.path().join("b.rs"))?.write_all(b"y")?;

    let mut app = App::new()?;
    app.left_panel.path = temp.path().to_path_buf();
    app.left_panel.load_entries()?;

    // Sort by extension
    app.set_overlay(Overlay::CommandLine("sort ext".into()));
    process_popup_input(&mut app, key(KeyCode::Enter));
    assert!(app.status_text().contains("Sort"));

    // Sort by size
    app.set_overlay(Overlay::CommandLine("sort size".into()));
    process_popup_input(&mut app, key(KeyCode::Enter));
    assert!(app.status_text().contains("Sort"));

    // Invalid sort mode
    app.set_overlay(Overlay::CommandLine("sort blah".into()));
    process_popup_input(&mut app, key(KeyCode::Enter));
    assert!(app.status_text().contains("sort expects") || app.status_text().contains("Error"));

    Ok(())
}

#[test]
fn test_command_line_mark_all_and_unmark() -> io::Result<()> {
    let temp = TempDir::new()?;
    File::create(temp.path().join("a.txt"))?.write_all(b"a")?;
    File::create(temp.path().join("b.txt"))?.write_all(b"b")?;

    let mut app = App::new()?;
    app.left_panel.path = temp.path().to_path_buf();
    app.left_panel.load_entries()?;

    // Mark all
    app.set_overlay(Overlay::CommandLine("mark all".into()));
    process_popup_input(&mut app, key(KeyCode::Enter));
    assert!(!app.left_panel.marked.is_empty());
    assert!(app.status_text().contains("Marked"));

    // Unmark
    app.set_overlay(Overlay::CommandLine("unmark".into()));
    process_popup_input(&mut app, key(KeyCode::Enter));
    assert!(app.left_panel.marked.is_empty());
    assert!(app.status_text().contains("Unmarked"));

    Ok(())
}

#[test]
fn test_command_line_help() -> io::Result<()> {
    let mut app = App::new()?;

    app.set_overlay(Overlay::CommandLine("help".into()));
    process_popup_input(&mut app, key(KeyCode::Enter));
    assert!(matches!(app.overlay, Overlay::Help));

    Ok(())
}

#[test]
fn test_command_line_quit() -> io::Result<()> {
    let mut app = App::new()?;

    app.set_overlay(Overlay::CommandLine("quit".into()));
    process_popup_input(&mut app, key(KeyCode::Enter));
    assert!(!app.lifecycle.is_running());

    Ok(())
}

#[test]
fn test_command_line_exit() -> io::Result<()> {
    let mut app = App::new()?;

    app.set_overlay(Overlay::CommandLine("exit".into()));
    process_popup_input(&mut app, key(KeyCode::Enter));
    assert!(!app.lifecycle.is_running());

    Ok(())
}

#[test]
fn test_command_line_empty_command() -> io::Result<()> {
    let mut app = App::new()?;

    app.set_overlay(Overlay::CommandLine("   ".into()));
    process_popup_input(&mut app, key(KeyCode::Enter));
    // Whitespace-only command is treated as empty — status may show error or remain unchanged
    // Just verify the command line was cleared
    assert!(app.get_command_line().is_none());

    Ok(())
}

#[test]
fn test_command_line_mark_invalid_subcommand() -> io::Result<()> {
    let mut app = App::new()?;

    app.set_overlay(Overlay::CommandLine("mark blah".into()));
    process_popup_input(&mut app, key(KeyCode::Enter));
    assert!(
        app.status_text().contains("Error") || app.status_text().contains("mark expects"),
        "Got: {}",
        app.status_text()
    );

    Ok(())
}

#[test]
fn test_command_line_sort_default_is_name() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel.path = temp.path().to_path_buf();
    app.left_panel.load_entries()?;

    // "sort" with no argument defaults to "name"
    app.set_overlay(Overlay::CommandLine("sort".into()));
    process_popup_input(&mut app, key(KeyCode::Enter));
    assert!(app.status_text().contains("Sort"));

    Ok(())
}
