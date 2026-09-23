//! Purpose: Strengthens handler assertions against mutation-test survivors.
//!
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use funny_file_manager::handlers::{
    handle_copy, handle_delete, handle_delete_confirm, handle_mkdir, handle_move, handle_view,
    process_popup_input,
};
use funny_file_manager::models::FileName;
use funny_file_manager::models::{App, Overlay, Viewer};
use funny_file_manager::scroll::ScrollTop;
use funny_file_manager::ui::ui;
use ratatui::backend::TestBackend;
use ratatui::Terminal;
use std::fs::{self, File};
use std::io::{self, Write};
use tempfile::TempDir;

const fn key_event(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::empty())
}

#[test]
fn test_process_popup_input_viewer_scroll() -> io::Result<()> {
    let mut app = App::new()?;

    // Create a viewer with multiple lines
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

    // Test scrolling down
    process_popup_input(&mut app, key_event(KeyCode::Down));
    assert_eq!(app.get_viewer().unwrap().scroll.lines(), 1);

    process_popup_input(&mut app, key_event(KeyCode::Down));
    assert_eq!(app.get_viewer().unwrap().scroll.lines(), 2);

    // Test scrolling up
    process_popup_input(&mut app, key_event(KeyCode::Up));
    assert_eq!(app.get_viewer().unwrap().scroll.lines(), 1);

    process_popup_input(&mut app, key_event(KeyCode::Up));
    assert_eq!(app.get_viewer().unwrap().scroll, ScrollTop::TOP);

    // Test scrolling up at top (should stay at 0)
    process_popup_input(&mut app, key_event(KeyCode::Up));
    assert_eq!(app.get_viewer().unwrap().scroll, ScrollTop::TOP);

    Ok(())
}

#[test]
fn test_process_popup_input_viewer_page_navigation() -> io::Result<()> {
    let mut app = App::new()?;

    // Create a viewer with many lines
    let lines: Vec<String> = (1..=100).map(|i| format!("Line {i}")).collect();
    app.set_overlay(Overlay::Viewer(Viewer::report("test.txt", lines)));

    // The page is the body height of the last frame, so draw one: 24 rows
    // less the status line leaves 23.
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).expect("test terminal");
    terminal.draw(|f| ui(f, &mut app)).expect("draw");

    process_popup_input(&mut app, key_event(KeyCode::PageDown));
    assert_eq!(app.get_viewer().unwrap().scroll.lines(), 23);
    process_popup_input(&mut app, key_event(KeyCode::PageUp));
    assert_eq!(app.get_viewer().unwrap().scroll, ScrollTop::TOP);

    // End stops with the last line on the bottom row, Home returns to the top.
    process_popup_input(&mut app, key_event(KeyCode::End));
    assert_eq!(app.get_viewer().unwrap().scroll.lines(), 77);
    process_popup_input(&mut app, key_event(KeyCode::Home));
    assert_eq!(app.get_viewer().unwrap().scroll, ScrollTop::TOP);

    Ok(())
}

#[test]
fn test_process_popup_input_command_line() -> io::Result<()> {
    let mut app = App::new()?;

    // Start command line
    app.set_overlay(Overlay::CommandLine(String::new().into()));

    // Type characters
    process_popup_input(&mut app, key_event(KeyCode::Char('c')));
    process_popup_input(&mut app, key_event(KeyCode::Char('d')));
    process_popup_input(&mut app, key_event(KeyCode::Char(' ')));
    process_popup_input(&mut app, key_event(KeyCode::Char('/')));

    assert_eq!(app.get_command_line(), Some("cd /"));

    // Test backspace
    process_popup_input(&mut app, key_event(KeyCode::Backspace));
    assert_eq!(app.get_command_line(), Some("cd "));

    // Test escape cancels
    process_popup_input(&mut app, key_event(KeyCode::Esc));
    assert!(app.get_command_line().is_none());

    Ok(())
}

#[test]
fn test_handle_view_with_file() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;

    // Create test file
    let file_path = temp_dir.path().join("test.txt");
    File::create(&file_path)?.write_all(b"Test content\nLine 2\nLine 3")?;

    app.left_panel.path = temp_dir.path().to_path_buf();
    app.left_panel.load_entries()?;

    // Select the file
    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == "test.txt" {
            app.left_panel.select(Some(i));
            break;
        }
    }

    handle_view(&mut app)?;

    assert!(app.get_viewer().is_some());
    let viewer = app.get_viewer().unwrap();
    assert_eq!(viewer.title(), "test.txt");
    assert_eq!(viewer.lines.len(), 3);
    assert_eq!(viewer.lines[0], "Test content");
    assert_eq!(viewer.scroll, ScrollTop::TOP);

    Ok(())
}

#[test]
fn test_handle_copy_file() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;

    let left_dir = temp_dir.path().join("left");
    let right_dir = temp_dir.path().join("right");
    fs::create_dir(&left_dir)?;
    fs::create_dir(&right_dir)?;

    // Create test file
    File::create(left_dir.join("test.txt"))?.write_all(b"content")?;

    app.left_panel.path = left_dir;
    app.right_panel.path = right_dir.clone();
    app.left_panel.load_entries()?;
    app.right_panel.load_entries()?;

    // Select the file
    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == "test.txt" {
            app.left_panel.select(Some(i));
            break;
        }
    }

    handle_copy(&mut app)?;

    // Verify file was copied
    assert!(right_dir.join("test.txt").exists());
    let content = fs::read_to_string(right_dir.join("test.txt"))?;
    assert_eq!(content, "content");

    Ok(())
}

#[test]
fn test_handle_move_file() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;

    let left_dir = temp_dir.path().join("left");
    let right_dir = temp_dir.path().join("right");
    fs::create_dir(&left_dir)?;
    fs::create_dir(&right_dir)?;

    // Create test file
    let src_file = left_dir.join("test.txt");
    File::create(&src_file)?.write_all(b"content")?;

    app.left_panel.path = left_dir;
    app.right_panel.path = right_dir.clone();
    app.left_panel.load_entries()?;
    app.right_panel.load_entries()?;

    // Select the file
    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == "test.txt" {
            app.left_panel.select(Some(i));
            break;
        }
    }

    handle_move(&mut app)?;

    // Verify file was moved
    assert!(!src_file.exists());
    assert!(right_dir.join("test.txt").exists());
    let content = fs::read_to_string(right_dir.join("test.txt"))?;
    assert_eq!(content, "content");

    Ok(())
}

#[test]
fn test_handle_mkdir() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;

    app.left_panel.path = temp_dir.path().to_path_buf();
    app.left_panel.load_entries()?;

    let initial_count = app.left_panel.entries.len();

    handle_mkdir(&mut app)?;

    // Verify directory was created (will be named "new_folder" or "new_folder_N")
    let new_dir = temp_dir.path().join("new_folder");
    assert!(new_dir.exists() || temp_dir.path().join("new_folder_1").exists());
    assert!(new_dir.is_dir() || temp_dir.path().join("new_folder_1").is_dir());

    // Verify panel was refreshed
    assert!(app.left_panel.entries.len() > initial_count);
    let names: Vec<_> = app.left_panel.entries.iter().map(|e| &e.name).collect();
    assert!(
        names.contains(&&"new_folder".to_owned()) || names.contains(&&"new_folder_1".to_owned())
    );

    Ok(())
}

#[test]
fn test_handle_delete_file() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;

    // Create test file
    let file_path = temp_dir.path().join("to_delete.txt");
    File::create(&file_path)?.write_all(b"delete me")?;

    app.left_panel.path = temp_dir.path().to_path_buf();
    app.left_panel.load_entries()?;

    // Select the file
    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == "to_delete.txt" {
            app.left_panel.select(Some(i));
            break;
        }
    }

    handle_delete_confirm(&mut app)?;
    handle_delete(&mut app)?;

    // Verify file was deleted
    assert!(!file_path.exists());

    // Verify panel was refreshed

    assert!(!app
        .left_panel
        .entries
        .iter()
        .map(|e| &e.name)
        .any(|x| x == &"to_delete.txt".to_owned()));

    Ok(())
}

#[test]
fn test_handle_delete_directory() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;

    // Create test directory with content
    let dir_path = temp_dir.path().join("to_delete");
    fs::create_dir(&dir_path)?;
    File::create(dir_path.join("file.txt"))?.write_all(b"content")?;

    app.left_panel.path = temp_dir.path().to_path_buf();
    app.left_panel.load_entries()?;

    // Select the directory
    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == "to_delete" {
            app.left_panel.select(Some(i));
            break;
        }
    }

    handle_delete_confirm(&mut app)?;
    handle_delete(&mut app)?;

    // Verify directory was deleted
    assert!(!dir_path.exists());

    // Verify panel was refreshed

    assert!(!app
        .left_panel
        .entries
        .iter()
        .map(|e| &e.name)
        .any(|x| x == &"to_delete".to_owned()));

    Ok(())
}

#[test]
fn test_handle_copy_single_file_only() -> io::Result<()> {
    // Note: handle_copy only copies the selected file, not marked files
    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;

    let left_dir = temp_dir.path().join("left");
    let right_dir = temp_dir.path().join("right");
    fs::create_dir(&left_dir)?;
    fs::create_dir(&right_dir)?;

    // Create test files
    File::create(left_dir.join("file1.txt"))?.write_all(b"content1")?;
    File::create(left_dir.join("file2.txt"))?.write_all(b"content2")?;

    app.left_panel.path = left_dir;
    app.right_panel.path = right_dir.clone();
    app.left_panel.load_entries()?;
    app.right_panel.load_entries()?;

    // Select file1.txt
    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == "file1.txt" {
            app.left_panel.select(Some(i));
            break;
        }
    }

    // If we mark file2.txt, it WILL be copied now because marked items are prioritized
    app.left_panel.marked.insert(FileName::from("file2.txt"));

    handle_copy(&mut app)?;

    // Verify marked file was copied, selected one (file1.txt) was NOT (because prioritization)
    assert!(!right_dir.join("file1.txt").exists());
    assert!(right_dir.join("file2.txt").exists());

    Ok(())
}
