use funny_file_manager::models::{App, Panel, SortMode, DirEntry};
use funny_file_manager::handlers::{handle_overlay_keys};
use funny_file_manager::utils::{format_size, copy_dir_all, unique_destination, refresh_panels};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::{fs, io};
use tempfile::TempDir;
use std::path::Path;

fn key_event(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
    KeyEvent::new(code, modifiers)
}

#[test]
fn test_format_size_all_ranges() {
    assert_eq!(format_size(0), "0 B");
    assert_eq!(format_size(1), "1 B");
    assert_eq!(format_size(999), "999 B");
    assert_eq!(format_size(1024), "1.0 KB");
    assert_eq!(format_size(1536), "1.5 KB");
    assert_eq!(format_size(1048576), "1.0 MB");
    assert_eq!(format_size(1572864), "1.5 MB");
    assert_eq!(format_size(1073741824), "1.0 GB");
    assert_eq!(format_size(1610612736), "1.5 GB");
    assert_eq!(format_size(1099511627776), "1.0 TB");
    assert_eq!(format_size(2199023255552), "2.0 TB");
}

#[test]
fn test_unique_destination_no_conflict() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let dest_path = temp_dir.path().join("new_file.txt");
    
    let result = unique_destination(&dest_path);
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
    
    let dest_path = temp_dir.path().join("file.txt");
    let result = unique_destination(&dest_path);
    
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
    fs::write(src_dir.join("subdir1").join("nested").join("file3.txt"), b"content3")?;
    
    copy_dir_all(&src_dir, &dst_dir)?;
    
    assert!(dst_dir.exists());
    assert!(dst_dir.join("file1.txt").exists());
    assert!(dst_dir.join("subdir1").join("file2.txt").exists());
    assert!(dst_dir.join("subdir1").join("nested").join("file3.txt").exists());
    
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
    app.right_panel = Panel::new(right_dir.clone())?;
    
    // Add a file after panels are created
    fs::write(left_dir.join("new_file.txt"), b"new content")?;
    
    refresh_panels(&mut app)?;
    
    // Check that the new file is loaded
    let has_new_file = app.left_panel.entries.iter().any(|e| e.name == "new_file.txt");
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
    panel.state.select(None);
    panel.ensure_selection();
    
    // Should select first item
    assert_eq!(panel.state.selected(), Some(0));
    
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
            panel.state.select(Some(i));
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
    panel.marked.insert("file1.txt".to_string());
    panel.marked.insert("file3.txt".to_string());
    
    // Select file2
    for (i, entry) in panel.entries.iter().enumerate() {
        if entry.name == "file2.txt" {
            panel.state.select(Some(i));
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
    app.viewer = Some(funny_file_manager::models::Viewer {
        title: "test.txt".to_string(),
        lines: vec!["Line 1".to_string(), "Line 2".to_string()],
        scroll: 0,
    });
    
    // Try to scroll down beyond limit
    handle_overlay_keys(&mut app, key_event(KeyCode::Down, KeyModifiers::empty()));
    handle_overlay_keys(&mut app, key_event(KeyCode::Down, KeyModifiers::empty()));
    handle_overlay_keys(&mut app, key_event(KeyCode::Down, KeyModifiers::empty()));
    
    // Should stop at max scroll
    assert!(app.viewer.as_ref().unwrap().scroll <= 1);
    
    Ok(())
}

#[test]
fn test_bookmarks_all_digits() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.show_bookmarks = true;
    
    // Test setting and going to all 10 bookmarks (0-9)
    for i in 0..10 {
        let digit = (i as u8 + b'0') as char;
        
        // Set bookmark
        handle_overlay_keys(&mut app, key_event(KeyCode::Char(digit), KeyModifiers::CONTROL));
        assert!(app.bookmarks[i].is_some());
        
        // Go to bookmark
        app.show_bookmarks = true;
        handle_overlay_keys(&mut app, key_event(KeyCode::Char(digit), KeyModifiers::empty()));
    }
    
    Ok(())
}

#[test]
fn test_command_line_cd_subdir() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let subdir = temp_dir.path().join("subdir");
    fs::create_dir(&subdir)?;
    
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.command_line = Some("cd subdir".to_string());
    
    handle_overlay_keys(&mut app, key_event(KeyCode::Enter, KeyModifiers::empty()));
    
    assert_eq!(app.left_panel.path.file_name().unwrap(), "subdir");
    
    Ok(())
}

#[test]
fn test_command_line_empty_enter() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.command_line = Some("".to_string());
    
    handle_overlay_keys(&mut app, key_event(KeyCode::Enter, KeyModifiers::empty()));
    
    assert!(app.command_line.is_none());
    
    Ok(())
}

#[test]
fn test_command_line_backspace() -> io::Result<()> {
    let mut app = App::new()?;
    app.command_line = Some("hello".to_string());
    
    handle_overlay_keys(&mut app, key_event(KeyCode::Backspace, KeyModifiers::empty()));
    assert_eq!(app.command_line, Some("hell".to_string()));
    
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
    
    assert!(matches!(app.active_panel, funny_file_manager::models::PanelSide::Left));
    app.switch_panel();
    assert!(matches!(app.active_panel, funny_file_manager::models::PanelSide::Right));
    app.switch_panel();
    assert!(matches!(app.active_panel, funny_file_manager::models::PanelSide::Left));
    
    Ok(())
}

#[test]
fn test_dir_entry_creation() {
    let entry = DirEntry {
        name: "test.txt".to_string(),
        is_dir: false,
        size: 1024,
    };
    
    assert_eq!(entry.name, "test.txt");
    assert!(!entry.is_dir);
    assert_eq!(entry.size, 1024);
}

#[test]
fn test_panel_wrap_around_navigation() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    for i in 1..=3 {
        fs::write(temp_dir.path().join(format!("file{}.txt", i)), b"test")?;
    }
    
    let mut panel = Panel::new(temp_dir.path().to_path_buf())?;
    
    // Go to last item
    panel.state.select(Some(panel.entries.len() - 1));
    
    // Next should wrap to first
    panel.next();
    assert_eq!(panel.state.selected(), Some(0));
    
    // Previous should wrap to last
    panel.previous();
    assert_eq!(panel.state.selected(), Some(panel.entries.len() - 1));
    
    Ok(())
}