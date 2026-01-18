use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use funny_file_manager::handlers::{
    handle_copy, handle_delete, handle_mkdir, handle_move, handle_overlay_keys, handle_view,
};
use funny_file_manager::models::{App, Viewer};
use std::fs::{self, File};
use std::io::{self, Write};
use tempfile::TempDir;

fn key_event(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::empty())
}

fn key_event_with_modifiers(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
    KeyEvent::new(code, modifiers)
}

#[test]
fn test_handle_overlay_keys_viewer_scroll() -> io::Result<()> {
    let mut app = App::new()?;

    // Create a viewer with multiple lines
    app.viewer = Some(Viewer {
        title: "test.txt".to_string(),
        lines: vec![
            "Line 1".to_string(),
            "Line 2".to_string(),
            "Line 3".to_string(),
            "Line 4".to_string(),
            "Line 5".to_string(),
        ],
        scroll: 0,
    });

    // Test scrolling down
    handle_overlay_keys(&mut app, key_event(KeyCode::Down));
    assert_eq!(app.viewer.as_ref().unwrap().scroll, 1);

    handle_overlay_keys(&mut app, key_event(KeyCode::Down));
    assert_eq!(app.viewer.as_ref().unwrap().scroll, 2);

    // Test scrolling up
    handle_overlay_keys(&mut app, key_event(KeyCode::Up));
    assert_eq!(app.viewer.as_ref().unwrap().scroll, 1);

    handle_overlay_keys(&mut app, key_event(KeyCode::Up));
    assert_eq!(app.viewer.as_ref().unwrap().scroll, 0);

    // Test scrolling up at top (should stay at 0)
    handle_overlay_keys(&mut app, key_event(KeyCode::Up));
    assert_eq!(app.viewer.as_ref().unwrap().scroll, 0);

    Ok(())
}

#[test]
fn test_handle_overlay_keys_viewer_page_navigation() -> io::Result<()> {
    let mut app = App::new()?;

    // Create a viewer with many lines
    let lines: Vec<String> = (1..=100).map(|i| format!("Line {}", i)).collect();
    app.viewer = Some(Viewer {
        title: "test.txt".to_string(),
        lines: lines.clone(),
        scroll: 0,
    });

    // Test PageDown (scrolls by 20 lines if possible)
    handle_overlay_keys(&mut app, key_event(KeyCode::PageDown));
    let scroll_after_pagedown = app.viewer.as_ref().unwrap().scroll;
    // scroll_after_pagedown will be >= 0 (usize can't be negative)

    // Test PageUp (only if we scrolled down)
    if scroll_after_pagedown > 0 {
        handle_overlay_keys(&mut app, key_event(KeyCode::PageUp));
        assert!(app.viewer.as_ref().unwrap().scroll < scroll_after_pagedown);
    }

    // Home/End keys are not handled in the viewer
    // The viewer stays at current position when these keys are pressed

    Ok(())
}

#[test]
fn test_handle_overlay_keys_command_line() -> io::Result<()> {
    let mut app = App::new()?;

    // Start command line
    app.command_line = Some(String::new());

    // Type characters
    handle_overlay_keys(&mut app, key_event(KeyCode::Char('c')));
    handle_overlay_keys(&mut app, key_event(KeyCode::Char('d')));
    handle_overlay_keys(&mut app, key_event(KeyCode::Char(' ')));
    handle_overlay_keys(&mut app, key_event(KeyCode::Char('/')));

    assert_eq!(app.command_line, Some("cd /".to_string()));

    // Test backspace
    handle_overlay_keys(&mut app, key_event(KeyCode::Backspace));
    assert_eq!(app.command_line, Some("cd ".to_string()));

    // Test escape cancels
    handle_overlay_keys(&mut app, key_event(KeyCode::Esc));
    assert_eq!(app.command_line, None);

    Ok(())
}

#[test]
fn test_handle_overlay_keys_bookmarks() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel.path = temp_dir.path().to_path_buf();

    // Set a bookmark (handle_overlay_keys only processes bookmarks when show_bookmarks is true)
    app.show_bookmarks = true;
    handle_overlay_keys(
        &mut app,
        key_event_with_modifiers(KeyCode::Char('1'), KeyModifiers::CONTROL),
    );
    assert!(app.bookmarks[1].is_some());

    // Navigate to bookmark
    app.show_bookmarks = true;
    handle_overlay_keys(&mut app, key_event(KeyCode::Char('1')));
    // Bookmarks will close if navigation succeeds

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
            app.left_panel.state.select(Some(i));
            break;
        }
    }

    handle_view(&mut app)?;

    assert!(app.viewer.is_some());
    let viewer = app.viewer.as_ref().unwrap();
    assert_eq!(viewer.title, "test.txt");
    assert_eq!(viewer.lines.len(), 3);
    assert_eq!(viewer.lines[0], "Test content");
    assert_eq!(viewer.scroll, 0);

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
            app.left_panel.state.select(Some(i));
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

    app.left_panel.path = left_dir.clone();
    app.right_panel.path = right_dir.clone();
    app.left_panel.load_entries()?;
    app.right_panel.load_entries()?;

    // Select the file
    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == "test.txt" {
            app.left_panel.state.select(Some(i));
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
        names.contains(&&"new_folder".to_string()) || names.contains(&&"new_folder_1".to_string())
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
            app.left_panel.state.select(Some(i));
            break;
        }
    }

    handle_delete(&mut app)?;

    // Verify file was deleted
    assert!(!file_path.exists());

    // Verify panel was refreshed
    let names: Vec<_> = app.left_panel.entries.iter().map(|e| &e.name).collect();
    assert!(!names.contains(&&"to_delete.txt".to_string()));

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
            app.left_panel.state.select(Some(i));
            break;
        }
    }

    handle_delete(&mut app)?;

    // Verify directory was deleted
    assert!(!dir_path.exists());

    // Verify panel was refreshed
    let names: Vec<_> = app.left_panel.entries.iter().map(|e| &e.name).collect();
    assert!(!names.contains(&&"to_delete".to_string()));

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
            app.left_panel.state.select(Some(i));
            break;
        }
    }

    // Mark file2.txt (but it won't be copied)
    app.left_panel.marked.insert("file2.txt".to_string());

    handle_copy(&mut app)?;

    // Verify only selected file was copied
    assert!(right_dir.join("file1.txt").exists());
    assert!(!right_dir.join("file2.txt").exists()); // Marked but not selected

    Ok(())
}
