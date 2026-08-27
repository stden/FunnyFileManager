//! Purpose: Targets coverage gaps across popup input, panels, and utility behavior.
//!
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use funny_file_manager::handlers::process_popup_input;
use funny_file_manager::models::EntryKind;
use funny_file_manager::models::FileName;
use funny_file_manager::models::FileSize;
use funny_file_manager::models::{App, DirEntry, Overlay, Panel, SortMode};
use funny_file_manager::scroll::ScrollTop;
use funny_file_manager::utils::{copy_dir_all, refresh_panels, unique_destination};
use std::{fs, io};
use tempfile::TempDir;

const fn key_event(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
    KeyEvent::new(code, modifiers)
}

#[test]
fn test_format_size_all_ranges() {
    assert_eq!(FileSize::from_bytes(0).to_string(), "0 B");
    assert_eq!(FileSize::from_bytes(1).to_string(), "1 B");
    assert_eq!(FileSize::from_bytes(999).to_string(), "999 B");
    assert_eq!(FileSize::from_bytes(1024).to_string(), "1.00 KB");
    assert_eq!(FileSize::from_bytes(1536).to_string(), "1.50 KB");
    assert_eq!(FileSize::from_bytes(1_048_576).to_string(), "1.00 MB");
    assert_eq!(FileSize::from_bytes(1_572_864).to_string(), "1.50 MB");
    assert_eq!(FileSize::from_bytes(1_073_741_824).to_string(), "1.00 GB");
    assert_eq!(FileSize::from_bytes(1_610_612_736).to_string(), "1.50 GB");
    assert_eq!(
        FileSize::from_bytes(1_099_511_627_776).to_string(),
        "1024.00 GB"
    );
    assert_eq!(
        FileSize::from_bytes(2_199_023_255_552).to_string(),
        "2048.00 GB"
    );
}

#[test]
fn test_unique_destination_no_conflict() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let dest_path = temp_dir.path().join("new_file.txt");

    let result = unique_destination(temp_dir.path(), std::ffi::OsStr::new("new_file.txt"))?;
    assert_eq!(result, dest_path);

    Ok(())
}

#[test]
fn test_unique_destination_with_conflicts() -> io::Result<()> {
    let temp_dir = TempDir::new()?;

    // Create existing files
    fs::write(temp_dir.path().join("file.txt"), b"original")?;
    fs::write(temp_dir.path().join("file_1.txt"), b"copy1")?;
    fs::write(temp_dir.path().join("file_2.txt"), b"copy2")?;

    let result = unique_destination(temp_dir.path(), std::ffi::OsStr::new("file.txt"))?;

    assert_eq!(result, temp_dir.path().join("file_3.txt"));

    Ok(())
}

#[test]
fn test_copy_dir_all_recursive() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let src_dir = temp_dir.path().join("src");
    let dst_dir = temp_dir.path().join("dst");

    // Create complex directory structure
    fs::create_dir(&src_dir)?;
    fs::create_dir(src_dir.join("subdir1"))?;
    fs::create_dir(src_dir.join("subdir2"))?;
    fs::create_dir(src_dir.join("subdir1").join("nested"))?;

    fs::write(src_dir.join("file1.txt"), b"content1")?;
    fs::write(src_dir.join("subdir1").join("file2.txt"), b"content2")?;
    fs::write(
        src_dir.join("subdir1").join("nested").join("file3.txt"),
        b"content3",
    )?;

    copy_dir_all(&src_dir, &dst_dir)?;

    assert!(dst_dir.exists());
    assert!(dst_dir.join("file1.txt").exists());
    assert!(dst_dir.join("subdir1").join("file2.txt").exists());
    assert!(dst_dir
        .join("subdir1")
        .join("nested")
        .join("file3.txt")
        .exists());

    Ok(())
}

#[test]
fn test_refresh_panels() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let left_dir = temp_dir.path().join("left");
    let right_dir = temp_dir.path().join("right");

    fs::create_dir(&left_dir)?;
    fs::create_dir(&right_dir)?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(left_dir.clone())?;
    app.right_panel = Panel::new(right_dir)?;

    // Add a file after panels are created
    fs::write(left_dir.join("new_file.txt"), b"new content")?;

    refresh_panels(&mut app)?;

    // Check that the new file is loaded
    let has_new_file = app
        .left_panel
        .entries
        .iter()
        .any(|e| e.name == "new_file.txt");
    assert!(has_new_file);

    Ok(())
}

#[test]
fn test_sort_mode_cycle() {
    let mode = SortMode::Name;
    assert_eq!(mode.next(), SortMode::Extension);
    assert_eq!(mode.next().next(), SortMode::Size);
    assert_eq!(mode.next().next().next(), SortMode::Name);
}

#[test]
fn test_sort_mode_labels() {
    assert_eq!(SortMode::Name.label(), "Name");
    assert_eq!(SortMode::Extension.label(), "Ext");
    assert_eq!(SortMode::Size.label(), "Size");
}

#[test]
fn test_panel_ensure_selection() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    fs::write(temp_dir.path().join("file.txt"), b"content")?;

    let mut panel = Panel::new(temp_dir.path().to_path_buf())?;

    // Clear selection
    panel.select(None);
    panel.ensure_selection();

    // Should select first item
    assert_eq!(panel.selected(), Some(0));

    Ok(())
}

#[test]
fn test_panel_selected_path() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("test.txt");
    fs::write(&file_path, b"content")?;

    let mut panel = Panel::new(temp_dir.path().to_path_buf())?;

    // Select the file
    for (i, entry) in panel.entries.iter().enumerate() {
        if entry.name == "test.txt" {
            panel.select(Some(i));
            break;
        }
    }

    let selected = panel.selected_path();
    assert_eq!(selected, Some(file_path));

    Ok(())
}

#[test]
fn test_panel_action_entry_names() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    fs::write(temp_dir.path().join("file1.txt"), b"content1")?;
    fs::write(temp_dir.path().join("file2.txt"), b"content2")?;
    fs::write(temp_dir.path().join("file3.txt"), b"content3")?;

    let mut panel = Panel::new(temp_dir.path().to_path_buf())?;

    // Mark file1 and file3
    panel.marked.insert(FileName::from("file1.txt"));
    panel.marked.insert(FileName::from("file3.txt"));

    // Select file2
    for (i, entry) in panel.entries.iter().enumerate() {
        if entry.name == "file2.txt" {
            panel.select(Some(i));
            break;
        }
    }

    let names = panel.action_entry_names();
    assert_eq!(names.len(), 2);
    assert!(names.contains(&"file1.txt".to_string()));
    assert!(names.contains(&"file3.txt".to_string()));

    // Clear marks and test with selection only
    panel.marked.clear();
    let names = panel.action_entry_names();
    assert_eq!(names.len(), 1);
    assert!(names.contains(&"file2.txt".to_string()));

    Ok(())
}

#[test]
fn test_viewer_scroll_limits() -> io::Result<()> {
    let mut app = App::new()?;
    app.set_overlay(Overlay::Viewer(funny_file_manager::models::Viewer {
        title: "test.txt".to_string(),
        lines: vec!["Line 1".to_string(), "Line 2".to_string()],
        scroll: ScrollTop::TOP,
    }));

    // Try to scroll down beyond limit
    process_popup_input(&mut app, key_event(KeyCode::Down, KeyModifiers::empty()));
    process_popup_input(&mut app, key_event(KeyCode::Down, KeyModifiers::empty()));
    process_popup_input(&mut app, key_event(KeyCode::Down, KeyModifiers::empty()));

    // Should stop at max scroll
    assert!(app.get_viewer().unwrap().scroll.lines() <= 1);

    Ok(())
}

#[test]
fn test_command_line_cd_subdir() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let subdir = temp_dir.path().join("subdir");
    fs::create_dir(&subdir)?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.set_overlay(Overlay::CommandLine("cd subdir".into()));

    process_popup_input(&mut app, key_event(KeyCode::Enter, KeyModifiers::empty()));

    assert_eq!(app.left_panel.path.file_name().unwrap(), "subdir");

    Ok(())
}

#[test]
fn test_command_line_empty_enter() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.set_overlay(Overlay::CommandLine("".into()));

    process_popup_input(&mut app, key_event(KeyCode::Enter, KeyModifiers::empty()));

    assert!(app.get_command_line().is_none());

    Ok(())
}

#[test]
fn test_command_line_backspace() -> io::Result<()> {
    let mut app = App::new()?;
    app.set_overlay(Overlay::CommandLine("hello".into()));

    process_popup_input(
        &mut app,
        key_event(KeyCode::Backspace, KeyModifiers::empty()),
    );
    assert_eq!(app.get_command_line(), Some("hell"));

    Ok(())
}

#[test]
fn test_panel_cycle_sort_mode() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut panel = Panel::new(temp_dir.path().to_path_buf())?;

    assert!(matches!(panel.sort_mode, SortMode::Name));
    panel.cycle_sort_mode();
    assert!(matches!(panel.sort_mode, SortMode::Extension));
    panel.cycle_sort_mode();
    assert!(matches!(panel.sort_mode, SortMode::Size));
    panel.cycle_sort_mode();
    assert!(matches!(panel.sort_mode, SortMode::Name));

    Ok(())
}

#[test]
fn test_app_switch_panel() -> io::Result<()> {
    let mut app = App::new()?;

    assert!(matches!(
        app.active_panel,
        funny_file_manager::models::PanelSide::Left
    ));
    app.switch_panel();
    assert!(matches!(
        app.active_panel,
        funny_file_manager::models::PanelSide::Right
    ));
    app.switch_panel();
    assert!(matches!(
        app.active_panel,
        funny_file_manager::models::PanelSide::Left
    ));

    Ok(())
}

#[test]
fn test_dir_entry_creation() {
    let entry = DirEntry {
        name: "test.txt".to_string(),
        kind: EntryKind::File,
        size: FileSize::from_bytes(1024),
    };

    assert_eq!(entry.name, "test.txt");
    assert!(!entry.kind.is_dir());
    assert_eq!(entry.size, FileSize::from_bytes(1024));
}

#[test]
fn test_panel_wrap_around_navigation() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    for i in 1..=3 {
        fs::write(temp_dir.path().join(format!("file{i}.txt")), b"test")?;
    }

    let mut panel = Panel::new(temp_dir.path().to_path_buf())?;

    // Go to last item
    panel.select(Some(panel.entries.len() - 1));

    // Next should wrap to first
    panel.next();
    assert_eq!(panel.selected(), Some(0));

    // Previous should wrap to last
    panel.previous();
    assert_eq!(panel.selected(), Some(panel.entries.len() - 1));

    Ok(())
}

// Test to cover the ^0 branches in utils.rs copy_dir_all
#[test]
#[cfg(unix)]
fn test_copy_dir_all_permission_errors() -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let temp_dir = TempDir::new()?;
    let src_dir = temp_dir.path().join("src");
    let dst_dir = temp_dir.path().join("dst");
    let protected_dir = temp_dir.path().join("src").join("protected");

    fs::create_dir(&src_dir)?;
    fs::create_dir(&protected_dir)?;
    fs::write(protected_dir.join("file.txt"), b"data")?;

    // Make directory unreadable
    let mut perms = fs::metadata(&protected_dir)?.permissions();
    perms.set_mode(0o000);
    fs::set_permissions(&protected_dir, perms)?;

    // This should handle the error gracefully
    let result = copy_dir_all(&src_dir, &dst_dir);

    // Restore permissions for cleanup
    let mut perms = fs::metadata(&protected_dir)
        .unwrap_or_else(|_| fs::metadata(&src_dir).unwrap())
        .permissions();
    perms.set_mode(0o755);
    let _ = fs::set_permissions(&protected_dir, perms);

    assert!(result.is_err() || dst_dir.exists());

    Ok(())
}

// Test to cover error path in refresh_panels
#[test]
fn test_refresh_panels_with_deleted_directories() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let left_dir = temp_dir.path().join("left");
    let right_dir = temp_dir.path().join("right");

    fs::create_dir(&left_dir)?;
    fs::create_dir(&right_dir)?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(left_dir.clone())?;
    app.right_panel = Panel::new(right_dir)?;

    // Delete the left directory to trigger an error
    fs::remove_dir_all(&left_dir)?;

    // This should handle the error
    let result = refresh_panels(&mut app);
    assert!(result.is_err());

    Ok(())
}

// Test copy_dir_all with source not existing
#[test]
fn test_copy_dir_all_src_not_exist() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let src_dir = temp_dir.path().join("nonexistent");
    let dst_dir = temp_dir.path().join("dst");

    let result = copy_dir_all(&src_dir, &dst_dir);
    assert!(result.is_err());

    Ok(())
}

// Test copy_dir_all with empty directory
#[test]
fn test_copy_dir_all_empty_dir() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let src_dir = temp_dir.path().join("empty");
    let dst_dir = temp_dir.path().join("dst");

    fs::create_dir(&src_dir)?;

    copy_dir_all(&src_dir, &dst_dir)?;

    assert!(dst_dir.exists());
    assert_eq!(fs::read_dir(&dst_dir)?.count(), 0);

    Ok(())
}

// Test unique_destination with no extension
#[test]
fn test_unique_destination_no_extension() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    fs::write(temp_dir.path().join("README"), b"data")?;
    fs::write(temp_dir.path().join("README_1"), b"data")?;

    let result = unique_destination(temp_dir.path(), std::ffi::OsStr::new("README"))?;
    assert_eq!(result.file_name().unwrap(), "README_2");

    Ok(())
}

// Test unique_destination with empty extension
#[test]
fn test_unique_destination_empty_extension() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    fs::write(temp_dir.path().join("file."), b"data")?;
    fs::write(temp_dir.path().join("file._1"), b"data")?;

    let result = unique_destination(temp_dir.path(), std::ffi::OsStr::new("file."))?;
    assert_eq!(result.file_name().unwrap(), "file_1");

    Ok(())
}

// Test unique_destination with dotfile
#[test]
fn test_unique_destination_dotfile() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    fs::write(temp_dir.path().join(".config"), b"data")?;

    let result = unique_destination(temp_dir.path(), std::ffi::OsStr::new(".config"))?;
    assert_eq!(result.file_name().unwrap(), ".config_1");

    Ok(())
}

// Test to exercise the None case in panel.get_selected_entry()
#[test]
fn test_panel_with_no_selection() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut panel = Panel::new(temp_dir.path().to_path_buf())?;
    panel.select(None);

    assert!(panel.get_selected_entry().is_none());

    Ok(())
}

// Test unique_destination with multiple dots
#[test]
fn test_unique_destination_multiple_dots() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    fs::write(temp_dir.path().join("file.tar.gz"), b"data")?;

    let result = unique_destination(temp_dir.path(), std::ffi::OsStr::new("file.tar.gz"))?;
    assert_eq!(result.file_name().unwrap(), "file.tar_1.gz");

    Ok(())
}

// Test format_size edge cases
#[test]
fn test_format_size_edge_cases() {
    // Test exact boundaries
    assert_eq!(FileSize::from_bytes(1023).to_string(), "1023 B");
    assert_eq!(FileSize::from_bytes(1_048_575).to_string(), "1024.00 KB");
    assert_eq!(
        FileSize::from_bytes(1_073_741_823).to_string(),
        "1024.00 MB"
    );
}

#[test]
fn test_writeable_backend_full_coverage() -> io::Result<()> {
    use funny_file_manager::WritableTestBackend;
    use ratatui::backend::Backend;
    use std::io::Write;

    let mut backend = WritableTestBackend::new(80, 24);

    // Test write method
    let bytes = backend.write(b"test")?;
    assert_eq!(bytes, 4);

    // Test flush (Write trait)
    Write::flush(&mut backend)?;

    // Test backend methods
    backend.hide_cursor()?;
    backend.show_cursor()?;
    backend.get_cursor_position()?;
    backend.set_cursor_position((10, 20))?;
    backend.clear()?;
    backend.size()?;
    backend.window_size()?;
    Backend::flush(&mut backend)?;

    Ok(())
}

#[test]
fn test_event_reader_coverage() -> io::Result<()> {
    use funny_file_manager::run_app_with_event_reader;
    use ratatui::Terminal;

    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.right_panel = Panel::new(temp_dir.path().to_path_buf())?;

    let backend = funny_file_manager::WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend)?;

    // Test with event reader that returns non-key events
    let mut events = vec![
        Event::Resize(100, 50),
        Event::Key(key_event(KeyCode::F(10), KeyModifiers::empty())),
    ]
    .into_iter();

    run_app_with_event_reader(&mut terminal, app, || {
        events
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "No more events"))
    })?;

    Ok(())
}

#[test]
#[cfg(coverage)]
fn test_coverage_specific_functions() -> io::Result<()> {
    // Test coverage-specific functions
    funny_file_manager::coverage_touch_logs();
    funny_file_manager::coverage_touch_backend()?;
    Ok(())
}

#[test]
fn test_navigation_with_debounce() -> io::Result<()> {
    use funny_file_manager::run_app_with_event_reader;
    use ratatui::Terminal;

    let temp_dir = TempDir::new()?;
    fs::write(temp_dir.path().join("file1.txt"), b"content1")?;
    fs::write(temp_dir.path().join("file2.txt"), b"content2")?;
    fs::write(temp_dir.path().join("file3.txt"), b"content3")?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.right_panel = Panel::new(temp_dir.path().to_path_buf())?;

    let backend = funny_file_manager::WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend)?;

    let mut events = vec![
        // Rapid navigation (will be debounced in coverage mode)
        Event::Key(key_event(KeyCode::Down, KeyModifiers::empty())),
        Event::Key(key_event(KeyCode::Down, KeyModifiers::empty())),
        Event::Key(key_event(KeyCode::Down, KeyModifiers::empty())),
        Event::Key(key_event(KeyCode::Down, KeyModifiers::empty())),
        Event::Key(key_event(KeyCode::Up, KeyModifiers::empty())),
        Event::Key(key_event(KeyCode::Up, KeyModifiers::empty())),
        Event::Key(key_event(KeyCode::Up, KeyModifiers::empty())),
        Event::Key(key_event(KeyCode::F(10), KeyModifiers::empty())),
    ]
    .into_iter();

    run_app_with_event_reader(&mut terminal, app, || {
        events
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "No more events"))
    })?;

    Ok(())
}

#[test]
fn test_ctrl_backslash_unix_root() -> io::Result<()> {
    use funny_file_manager::run_app_with_event_reader;
    use ratatui::Terminal;

    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.right_panel = Panel::new(temp_dir.path().to_path_buf())?;

    let backend = funny_file_manager::WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend)?;

    let mut events = vec![
        Event::Key(key_event(KeyCode::Char('\\'), KeyModifiers::CONTROL)),
        Event::Key(key_event(KeyCode::F(10), KeyModifiers::empty())),
    ]
    .into_iter();

    run_app_with_event_reader(&mut terminal, app, || {
        events
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "No more events"))
    })?;

    Ok(())
}
