//! Purpose: Built-in command-line commands: pwd, exit, sort, mark, help and unknown input.

use super::*;

#[test]
fn test_command_line_pwd() -> io::Result<()> {
    let temp_dir = TempDir::new()?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.set_overlay(Overlay::CommandLine("pwd".into()));

    process_popup_input(&mut app, key_event(KeyCode::Enter));
    assert!(app.get_command_line().is_none());
    assert!(app
        .status_text()
        .contains(temp_dir.path().to_str().unwrap()));

    Ok(())
}

#[test]
fn test_command_line_exit() -> io::Result<()> {
    let temp_dir = TempDir::new()?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.set_overlay(Overlay::CommandLine("exit".into()));

    process_popup_input(&mut app, key_event(KeyCode::Enter));
    assert!(!app.lifecycle.is_running());

    Ok(())
}

#[test]
fn test_command_line_unknown_command() -> io::Result<()> {
    let temp_dir = TempDir::new()?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.set_overlay(Overlay::CommandLine("unknowncommand".into()));

    process_popup_input(&mut app, key_event(KeyCode::Enter));
    // Unknown commands are run as shell commands; the status will reflect
    // the shell's error output (e.g. "not found") or exit code.
    assert!(
        !app.status_text().is_empty(),
        "status_message should contain shell output for unknown command"
    );

    Ok(())
}

#[test]
fn test_command_line_sort_extension() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    File::create(temp_dir.path().join("a.txt"))?;
    File::create(temp_dir.path().join("b.rs"))?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.set_overlay(Overlay::CommandLine("sort extension".into()));

    process_popup_input(&mut app, key_event(KeyCode::Enter));
    assert!(!app.status_text().is_empty());

    Ok(())
}

#[test]
fn test_command_line_sort_size_right_panel() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    File::create(temp_dir.path().join("small.txt"))?.write_all(b"1")?;
    File::create(temp_dir.path().join("large.txt"))?.write_all(b"12345")?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.right_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.active_panel = PanelSide::Right;
    app.set_overlay(Overlay::CommandLine("sort size".into()));

    process_popup_input(&mut app, key_event(KeyCode::Enter));
    assert!(matches!(app.right_panel.sort_mode, SortMode::Size));
    assert!(app.status_text().contains("Sort: Size"));

    Ok(())
}

#[test]
fn test_command_line_sort_invalid() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    File::create(temp_dir.path().join("a.txt"))?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.set_overlay(Overlay::CommandLine("sort nope".into()));

    process_popup_input(&mut app, key_event(KeyCode::Enter));
    assert!(!app.status_text().is_empty());

    Ok(())
}

#[test]
fn test_command_line_mark_all() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    File::create(temp_dir.path().join("a.txt"))?;
    File::create(temp_dir.path().join("b.txt"))?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.set_overlay(Overlay::CommandLine("mark all".into()));

    process_popup_input(&mut app, key_event(KeyCode::Enter));
    assert!(!app.left_panel.marked.is_empty());
    assert!(app.status_text().contains("Marked"));

    Ok(())
}

#[test]
fn test_command_line_unmark() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    File::create(temp_dir.path().join("a.txt"))?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.left_panel.marked.insert(FileName::from("a.txt"));
    app.set_overlay(Overlay::CommandLine("unmark".into()));

    process_popup_input(&mut app, key_event(KeyCode::Enter));
    assert!(app.left_panel.marked.is_empty());
    assert!(app.status_text().contains("Unmarked"));

    Ok(())
}

#[test]
fn test_command_line_help() -> io::Result<()> {
    let temp_dir = TempDir::new()?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.set_overlay(Overlay::CommandLine("help".into()));

    process_popup_input(&mut app, key_event(KeyCode::Enter));
    assert!(matches!(app.overlay, Overlay::Help));

    Ok(())
}
