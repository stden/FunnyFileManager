//! Purpose: Adds extra integration coverage for file operations and popup handling.
//!
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use funny_file_manager::handlers::{
    handle_copy, handle_delete_confirm, handle_mkdir, handle_move, process_popup_input,
};
use funny_file_manager::models::FileName;
use funny_file_manager::models::{App, Overlay, Panel, PanelSide, SortMode};
use std::fs::{self, File};
use std::io;
use std::path::PathBuf;
use tempfile::TempDir;

const fn key_event(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::empty())
}

#[test]
fn test_cmd_cd_invalid_path() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;

    // Non-existent path should fail canonicalize
    app.set_overlay(Overlay::CommandLine("cd non_existent_path_xyz".into()));
    process_popup_input(&mut app, key_event(KeyCode::Enter));

    assert!(app.status_text().contains("Error"));
    Ok(())
}

#[test]
fn test_cmd_cd_to_file() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("test.txt");
    File::create(&file_path)?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;

    app.set_overlay(Overlay::CommandLine(
        format!("cd {}", file_path.display()).into(),
    ));
    process_popup_input(&mut app, key_event(KeyCode::Enter));

    assert!(app.status_text().contains("Error"));
    assert!(app.status_text().contains("not a directory"));
    Ok(())
}

#[test]
fn test_cmd_bookmark_invalid_slot() -> io::Result<()> {
    let mut app = App::new()?;
    app.set_overlay(Overlay::CommandLine("bookmark set 10".into()));
    process_popup_input(&mut app, key_event(KeyCode::Enter));
    assert!(app.status_text().contains("Error"));

    app.set_overlay(Overlay::CommandLine("bookmark set abc".into()));
    process_popup_input(&mut app, key_event(KeyCode::Enter));
    assert!(app.status_text().contains("Error"));
    Ok(())
}

#[test]
fn test_cmd_bookmark_go_empty() -> io::Result<()> {
    let mut app = App::new()?;
    app.set_overlay(Overlay::CommandLine("bookmark go 5".into()));
    process_popup_input(&mut app, key_event(KeyCode::Enter));
    assert!(app.status_text().contains("Error"));
    assert!(app.status_text().contains("empty"));
    Ok(())
}

#[test]
fn test_process_popup_input_drives() -> io::Result<()> {
    let mut app = App::new()?;
    app.set_overlay(Overlay::DriveSelection {
        side: PanelSide::Left,
        selected_index: 0,
        rows: funny_file_manager::models::drive_rows(),
    });
    let drives = App::get_all_drives();
    let drive_count = drives.len();

    for (code, expected) in [
        (KeyCode::Down, usize::from(drive_count > 1)),
        (KeyCode::Up, 0),
        (KeyCode::Up, drive_count.saturating_sub(1)),
        (KeyCode::End, drive_count.saturating_sub(1)),
        (KeyCode::Home, 0),
    ] {
        process_popup_input(&mut app, key_event(code));
        assert!(matches!(app.overlay,
            Overlay::DriveSelection { selected_index, .. } if selected_index == expected));
    }

    // Test Esc
    process_popup_input(&mut app, key_event(KeyCode::Esc));
    assert!(!matches!(app.overlay, Overlay::DriveSelection { .. }));

    Ok(())
}

#[test]
#[cfg(unix)]
fn test_handle_view_io_error() -> io::Result<()> {
    use funny_file_manager::handlers::handle_view;
    use std::os::unix::fs::PermissionsExt;

    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("unreadable.txt");
    File::create(&file_path)?;

    fs::set_permissions(&file_path, fs::Permissions::from_mode(0o000))?;
    // Root bypasses Unix DAC permissions, so a 0o000 file may still be readable;
    // only assert the IO error when permissions are actually enforced.
    let perms_enforced = fs::read(&file_path).is_err();

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;

    if let Some(i) = app
        .left_panel
        .entries
        .iter()
        .position(|e| e.name == "unreadable.txt")
    {
        app.left_panel.select(Some(i));
        let result = handle_view(&mut app);
        if perms_enforced {
            assert!(result.is_err());
        }
    }

    fs::set_permissions(&file_path, fs::Permissions::from_mode(0o644))?;
    Ok(())
}

#[test]
fn test_panel_new_invalid_path() {
    let result = Panel::new(PathBuf::from("/non/existent/path/that/should/fail"));
    assert!(result.is_err());
}

#[test]
fn test_process_popup_input_new_file_input() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.set_overlay(Overlay::NewFileInput("newfile.txt".to_owned()));

    // Test typing
    process_popup_input(&mut app, key_event(KeyCode::Char('2')));
    assert_eq!(app.get_new_file_input(), Some("newfile.txt2"));

    // Test backspace
    process_popup_input(&mut app, key_event(KeyCode::Backspace));
    assert_eq!(app.get_new_file_input(), Some("newfile.txt"));

    // Test Enter to create file
    process_popup_input(&mut app, key_event(KeyCode::Enter));
    assert!(app.get_new_file_input().is_none());
    assert!(temp_dir.path().join("newfile.txt").exists());
    assert!(app.status_text().contains("Created file"));

    // Test create already existing file
    app.set_overlay(Overlay::NewFileInput("newfile.txt".to_owned()));
    process_popup_input(&mut app, key_event(KeyCode::Enter));
    assert!(app.status_text().contains("already exists"));

    // Test Esc
    app.set_overlay(Overlay::NewFileInput("anything".to_owned()));
    process_popup_input(&mut app, key_event(KeyCode::Esc));
    assert!(app.get_new_file_input().is_none());

    Ok(())
}

#[test]
fn test_handle_delete_confirm_parent_dir() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;

    // Select ".." entry
    let dotdot_index = app
        .left_panel
        .entries
        .iter()
        .position(|e| e.name == "..")
        .unwrap();
    app.left_panel.select(Some(dotdot_index));

    handle_delete_confirm(&mut app)?;
    assert!(app.get_delete_confirm().is_none());

    Ok(())
}

#[test]
fn test_handle_delete_cancel() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("keep_me.txt");
    File::create(&file_path)?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.left_panel.select(Some(1)); // Select the file

    handle_delete_confirm(&mut app)?;
    assert!(app.get_delete_confirm().is_some());

    // Cancel with 'n'
    process_popup_input(&mut app, key_event(KeyCode::Char('n')));
    assert!(app.get_delete_confirm().is_none());
    assert!(file_path.exists());
    assert!(app.status_text().contains("cancelled"));

    Ok(())
}

#[test]
fn test_cmd_mark_invalid() -> io::Result<()> {
    let mut app = App::new()?;
    app.set_overlay(Overlay::CommandLine("mark unknown".into()));
    process_popup_input(&mut app, key_event(KeyCode::Enter));
    assert!(app.status_text().contains("Error"));
    Ok(())
}

#[test]
fn test_handle_copy_marked_files() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let left_dir = temp_dir.path().join("left");
    let right_dir = temp_dir.path().join("right");
    fs::create_dir(&left_dir)?;
    fs::create_dir(&right_dir)?;

    File::create(left_dir.join("a.txt"))?;
    File::create(left_dir.join("b.txt"))?;
    File::create(left_dir.join("c.txt"))?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(left_dir)?;
    app.right_panel = Panel::new(right_dir.clone())?;

    // Mark a.txt and c.txt
    app.left_panel.marked.insert(FileName::from("a.txt"));
    app.left_panel.marked.insert(FileName::from("c.txt"));

    handle_copy(&mut app)?;

    assert!(right_dir.join("a.txt").exists());
    assert!(!right_dir.join("b.txt").exists());
    assert!(right_dir.join("c.txt").exists());
    assert!(app.status_text().contains("Copied 2 item(s)"));

    Ok(())
}

#[test]
fn test_handle_move_marked_files() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let left_dir = temp_dir.path().join("left");
    let right_dir = temp_dir.path().join("right");
    fs::create_dir(&left_dir)?;
    fs::create_dir(&right_dir)?;

    File::create(left_dir.join("a.txt"))?;
    File::create(left_dir.join("b.txt"))?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(left_dir.clone())?;
    app.right_panel = Panel::new(right_dir.clone())?;

    // Mark a.txt
    app.left_panel.marked.insert(FileName::from("a.txt"));

    handle_move(&mut app)?;

    assert!(!left_dir.join("a.txt").exists());
    assert!(right_dir.join("a.txt").exists());
    assert!(left_dir.join("b.txt").exists());

    Ok(())
}

#[test]
fn test_handle_mkdir_failure() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    // Create a file where we want to create a directory
    let file_path = temp_dir.path().join("blocking_file");
    File::create(&file_path)?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;

    // Try to mkdir inside the file path (which is impossible)
    app.left_panel.path = file_path;

    // This should fail
    let result = handle_mkdir(&mut app);
    assert!(result.is_err());

    Ok(())
}

#[test]
fn test_cmd_mark_all_empty() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    // Directory with only ".."
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    // Clear any files if App::new used current dir
    app.left_panel.entries.retain(|e| e.name == "..");

    app.set_overlay(Overlay::CommandLine("mark all".into()));
    process_popup_input(&mut app, key_event(KeyCode::Enter));

    assert!(app.left_panel.marked.is_empty());
    Ok(())
}

#[test]
fn test_sort_mode_methods() {
    let mode = SortMode::Name;
    assert_eq!(mode.label(), "Name");
    let next = mode.next();
    assert_eq!(next, SortMode::Extension);
    assert_eq!(next.label(), "Ext");
    let next2 = next.next();
    assert_eq!(next2, SortMode::Size);
    assert_eq!(next2.label(), "Size");
    let next3 = next2.next();
    assert_eq!(next3, SortMode::Name);
}

#[test]
fn test_panel_select_after_reload_gone() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    File::create(temp_dir.path().join("a.txt"))?;
    File::create(temp_dir.path().join("b.txt"))?;

    let mut panel = Panel::new(temp_dir.path().to_path_buf())?;
    // Select b.txt
    let b_index = panel
        .entries
        .iter()
        .position(|e| e.name == "b.txt")
        .unwrap();
    panel.select(Some(b_index));

    // Delete b.txt and reload
    fs::remove_file(temp_dir.path().join("b.txt"))?;
    panel.load_entries()?;

    // Should fallback to first entry (or .. if present)
    assert!(panel.selected().is_some());
    assert_ne!(panel.entries[panel.selected().unwrap()].name, "b.txt");

    Ok(())
}

#[test]
fn test_full_event_loop_keys() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    File::create(temp_dir.path().join("file1.txt"))?;
    File::create(temp_dir.path().join("file2.txt"))?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.right_panel = Panel::new(temp_dir.path().to_path_buf())?;

    let mut terminal =
        ratatui::Terminal::new(funny_file_manager::WritableTestBackend::new(80, 24))?;

    let mut events = workflow_keys();

    funny_file_manager::run_app_with_event_reader(&mut terminal, app, || {
        events
            .next()
            .ok_or_else(|| io::Error::other("no more events"))
    })?;

    Ok(())
}

fn workflow_keys() -> std::vec::IntoIter<Event> {
    vec![
        // Command line
        Event::Key(key_event(KeyCode::Char(':'))),
        Event::Key(key_event(KeyCode::Char('p'))),
        Event::Key(key_event(KeyCode::Char('w'))),
        Event::Key(key_event(KeyCode::Char('d'))),
        Event::Key(key_event(KeyCode::Enter)),
        // Marking with Space
        Event::Key(key_event(KeyCode::Down)),
        Event::Key(key_event(KeyCode::Char(' '))),
        // Marking with Insert
        Event::Key(key_event(KeyCode::Insert)),
        // Navigation
        Event::Key(key_event(KeyCode::Home)),
        Event::Key(key_event(KeyCode::End)),
        // Ctrl+R Refresh
        Event::Key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL)),
        // Ctrl+U Swap
        Event::Key(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL)),
        // Ctrl+O Console
        Event::Key(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::CONTROL)),
        // Ctrl+\ Root
        Event::Key(KeyEvent::new(KeyCode::Char('\\'), KeyModifiers::CONTROL)),
        // Shift+F4 New File
        Event::Key(KeyEvent::new(KeyCode::F(4), KeyModifiers::SHIFT)),
        Event::Key(key_event(KeyCode::Esc)),
        // Alt+F1 Drives
        Event::Key(KeyEvent::new(KeyCode::F(1), KeyModifiers::ALT)),
        Event::Key(key_event(KeyCode::Esc)),
        // F1 Help
        Event::Key(key_event(KeyCode::F(1))),
        Event::Key(key_event(KeyCode::F(1))),
        // Quit
        Event::Key(key_event(KeyCode::F(10))),
        // F10 now asks before quitting; Enter answers Yes.
        Event::Key(key_event(KeyCode::Enter)),
    ]
    .into_iter()
}
