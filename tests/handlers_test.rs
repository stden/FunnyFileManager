//! Purpose: Covers handler workflows at the integration-test level.
//!
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use funny_file_manager::handlers::{
    handle_copy, handle_delete, handle_delete_confirm, handle_mkdir, handle_move, handle_view,
    process_popup_input,
};
use funny_file_manager::models::FileName;
use funny_file_manager::models::{App, Overlay, Panel, PanelSide, SortMode, Viewer};
use funny_file_manager::scroll::ScrollTop;
use std::fs::{self, File};
use std::io::{self, Write};
use tempfile::TempDir;

const fn key_event(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::empty())
}

#[test]
fn test_cd_dotdot() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let subdir = temp_dir.path().join("subdir");
    fs::create_dir(&subdir)?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(subdir)?;

    app.set_overlay(Overlay::CommandLine("cd ..".into()));
    process_popup_input(&mut app, key_event(KeyCode::Enter));

    assert_eq!(
        app.left_panel.path.canonicalize()?,
        temp_dir.path().canonicalize()?
    );

    Ok(())
}

#[test]
fn test_viewer_scroll_down_up() -> io::Result<()> {
    let mut app = App::new()?;
    app.set_overlay(Overlay::Viewer(Viewer::report(
        "test.txt",
        vec![
            "Line 1".to_owned(),
            "Line 2".to_owned(),
            "Line 3".to_owned(),
            "Line 4".to_owned(),
            "Line 5".to_owned(),
        ],
    )));
    app.set_viewport(funny_file_manager::models::Viewport::new(2));

    // Scroll down
    process_popup_input(&mut app, key_event(KeyCode::Down));
    assert_eq!(app.get_viewer().unwrap().scroll.lines(), 1);

    // Scroll up
    process_popup_input(&mut app, key_event(KeyCode::Up));
    assert_eq!(app.get_viewer().unwrap().scroll, ScrollTop::TOP);

    // Scroll up at 0 stays at 0
    process_popup_input(&mut app, key_event(KeyCode::Up));
    assert_eq!(app.get_viewer().unwrap().scroll, ScrollTop::TOP);

    Ok(())
}

#[test]
fn test_viewer_close_with_f3() -> io::Result<()> {
    let mut app = App::new()?;
    app.set_overlay(Overlay::Viewer(Viewer::report(
        "test.txt",
        vec!["Line 1".to_owned()],
    )));

    process_popup_input(&mut app, key_event(KeyCode::F(3)));
    assert!(app.get_viewer().is_none());

    Ok(())
}

#[test]
fn test_help_toggle_with_f1() -> io::Result<()> {
    let mut app = App::new()?;
    app.set_overlay(Overlay::Help);

    process_popup_input(&mut app, key_event(KeyCode::F(1)));
    assert!(!matches!(app.overlay, Overlay::Help));

    Ok(())
}

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
fn test_handle_view_text_file() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("test.txt");
    File::create(&file_path)?.write_all(b"Hello World")?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.right_panel = Panel::new(temp_dir.path().to_path_buf())?;

    // Select the file
    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == "test.txt" {
            app.left_panel.select(Some(i));
            break;
        }
    }

    handle_view(&mut app)?;

    assert!(app.get_viewer().is_some());
    assert_eq!(app.get_viewer().unwrap().title(), "test.txt");
    assert!(app.get_viewer().unwrap().lines[0].contains("Hello World"));

    Ok(())
}

#[test]
fn test_handle_copy_file() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let left_dir = temp_dir.path().join("left");
    let right_dir = temp_dir.path().join("right");
    fs::create_dir(&left_dir)?;
    fs::create_dir(&right_dir)?;

    let source_file = left_dir.join("source.txt");
    File::create(&source_file)?.write_all(b"content")?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(left_dir)?;
    app.right_panel = Panel::new(right_dir.clone())?;

    // Select the file
    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == "source.txt" {
            app.left_panel.select(Some(i));
            break;
        }
    }

    handle_copy(&mut app)?;

    assert!(right_dir.join("source.txt").exists());
    assert!(app.status_text().contains("Copied to"));

    Ok(())
}

#[test]
fn test_handle_move_file() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let left_dir = temp_dir.path().join("left");
    let right_dir = temp_dir.path().join("right");
    fs::create_dir(&left_dir)?;
    fs::create_dir(&right_dir)?;

    let source_file = left_dir.join("moveme.txt");
    File::create(&source_file)?.write_all(b"content")?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(left_dir.clone())?;
    app.right_panel = Panel::new(right_dir.clone())?;

    // Select the file
    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == "moveme.txt" {
            app.left_panel.select(Some(i));
            break;
        }
    }

    handle_move(&mut app)?;

    assert!(!left_dir.join("moveme.txt").exists());
    assert!(right_dir.join("moveme.txt").exists());
    assert!(app.status_text().contains("Moved to"));

    Ok(())
}

#[test]
fn test_handle_mkdir_creates_folder() -> io::Result<()> {
    let temp_dir = TempDir::new()?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.right_panel = Panel::new(temp_dir.path().to_path_buf())?;

    handle_mkdir(&mut app)?;

    assert!(temp_dir.path().join("new_folder").exists());
    assert!(app.status_text().contains("Created directory"));

    Ok(())
}

#[test]
fn test_handle_delete_file() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("delete_me.txt");
    File::create(&file_path)?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.right_panel = Panel::new(temp_dir.path().to_path_buf())?;

    // Select the file
    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == "delete_me.txt" {
            app.left_panel.select(Some(i));
            break;
        }
    }

    handle_delete_confirm(&mut app)?;
    handle_delete(&mut app)?;

    assert!(!file_path.exists());
    assert!(app.status_text().contains("Deleted"));

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

#[test]
fn test_command_line_shell_stdout() -> io::Result<()> {
    // Kill: stdout branch — last line of stdout shown as status
    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;

    app.set_overlay(Overlay::CommandLine("echo hello".into()));
    process_popup_input(&mut app, key_event(KeyCode::Enter));
    assert!(
        app.status_text().contains("hello"),
        "status: {}",
        app.status_text()
    );

    Ok(())
}

#[test]
fn test_command_line_shell_silent_success() -> io::Result<()> {
    // Kill: silent-success branch — "OK: <cmd>" shown when no stdout/stderr, exit 0
    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;

    // "rem" is a CMD remark (Windows) / comment (Unix sh) — exits 0, no output,
    // and is NOT intercepted by any internal command handler.
    #[cfg(windows)]
    let cmd = "rem silence";
    #[cfg(not(windows))]
    let cmd = "true";

    app.set_overlay(Overlay::CommandLine(cmd.into()));
    process_popup_input(&mut app, key_event(KeyCode::Enter));
    assert!(
        app.status_text().starts_with("OK:"),
        "expected 'OK:' prefix, got: {}",
        app.status_text()
    );

    Ok(())
}

#[test]
#[cfg(not(windows))]
fn test_command_line_shell_exit_failure() -> io::Result<()> {
    // Kill: exit-failure branch — "Exit <code>" shown when non-zero exit, no output
    // Unix-only: `false` exits 1 with no stdout/stderr and is not an internal command.
    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;

    app.set_overlay(Overlay::CommandLine("false".into()));
    process_popup_input(&mut app, key_event(KeyCode::Enter));
    assert!(
        app.status_text().starts_with("Exit"),
        "expected 'Exit' prefix, got: {}",
        app.status_text()
    );

    Ok(())
}
