//! Unit tests: panel navigation (split out to keep files under 500 lines).
#![allow(unused_imports)]

use ratatui::widgets::ListState;
use std::collections::BTreeSet;
use std::fs;

use super::tests::*;

use super::*;

use std::fs::File;

use std::io::Write;

use tempfile::TempDir;

#[test]
fn test_panel_load_entries_with_files() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");

    // Create test files and directories
    fs::create_dir(temp_dir.path().join("dir1")).expect("test setup");
    fs::create_dir(temp_dir.path().join("dir2")).expect("test setup");
    File::create(temp_dir.path().join("file1.txt"))
        .expect("test setup")
        .write_all(b"test")
        .expect("test setup");
    File::create(temp_dir.path().join("file2.txt"))
        .expect("test setup")
        .write_all(b"test")
        .expect("test setup");

    let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    panel.load_entries().expect("test setup");

    // Check entries are sorted (directories first, then files)
    let mut found_file = false;
    let mut found_dir_after_file = false;
    for entry in &panel.entries {
        if entry.name != ".." {
            found_dir_after_file |= entry.kind.is_dir() && found_file;
            found_file |= !entry.kind.is_dir();
        }
    }
    assert!(
        !found_dir_after_file,
        "Found directory after file in sorted entries"
    );

    // Verify we have the expected entries
    assert!(panel.entries.len() >= 5); // .. + 2 dirs + 2 files
    assert!(panel
        .entries
        .iter()
        .any(|e| e.name == "dir1" && e.kind.is_dir()));
    assert!(panel
        .entries
        .iter()
        .any(|e| e.name == "dir2" && e.kind.is_dir()));
    assert!(panel
        .entries
        .iter()
        .any(|e| e.name == "file1.txt" && !e.kind.is_dir()));
    assert!(panel
        .entries
        .iter()
        .any(|e| e.name == "file2.txt" && !e.kind.is_dir()));

    Ok(())
}

#[test]
fn test_panel_navigation_next() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");

    // Create multiple files
    for i in 0..5 {
        File::create(temp_dir.path().join(format!("file{i}.txt"))).expect("test setup");
    }

    let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    // Test next navigation
    assert_eq!(panel.selected(), Some(0));

    panel.next();
    assert_eq!(panel.selected(), Some(1));

    panel.next();
    assert_eq!(panel.selected(), Some(2));

    // Navigate to last item
    while panel.selected() != Some(panel.entries.len() - 1) {
        panel.next();
    }

    // Next should wrap to first
    panel.next();
    assert_eq!(panel.selected(), Some(0));

    Ok(())
}

#[test]
fn test_panel_navigation_previous() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");

    // Create multiple files
    for i in 0..5 {
        File::create(temp_dir.path().join(format!("file{i}.txt"))).expect("test setup");
    }

    let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    // Test previous navigation from first item (should wrap to last)
    panel.previous();
    assert_eq!(panel.selected(), Some(panel.entries.len() - 1));

    // Navigate back
    panel.previous();
    assert_eq!(panel.selected(), Some(panel.entries.len() - 2));

    Ok(())
}

#[test]
fn test_panel_navigation_empty() {
    let mut panel = Panel::from_entries(PathBuf::new(), Vec::new());

    // Navigation should not crash on empty panel
    panel.next();
    assert_eq!(panel.selected(), None);

    panel.previous();
    assert_eq!(panel.selected(), None);
}

#[test]
fn test_panel_enter_directory() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let subdir = temp_dir.path().join("subdir");
    fs::create_dir(&subdir).expect("test setup");

    let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    // Find and select the subdirectory
    for (i, entry) in panel.entries.iter().enumerate() {
        if entry.name == "subdir" {
            panel.select(Some(i));
            break;
        }
    }

    // Enter the directory
    panel.enter().expect("test setup");
    assert_eq!(panel.path, subdir);

    // Should have .. entry
    assert!(panel.entries.iter().any(|e| e.name == ".."));

    // Go back to parent
    panel.select(Some(0)); // Select ..
    panel.enter().expect("test setup");
    assert_eq!(panel.path, temp_dir.path());

    Ok(())
}

#[test]
fn test_panel_enter_clears_marks() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let subdir = temp_dir.path().join("subdir");
    fs::create_dir(&subdir).expect("test setup");
    File::create(temp_dir.path().join("file.txt")).expect("test setup");

    let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    let file_index = panel
        .entries
        .iter()
        .position(|entry| entry.name == "file.txt")
        .expect("file.txt entry");
    panel.select(Some(file_index));
    assert!(panel.toggle_mark_selected());
    assert!(panel.marked.contains("file.txt"));

    let dir_index = panel
        .entries
        .iter()
        .position(|entry| entry.name == "subdir")
        .expect("subdir entry");
    panel.select(Some(dir_index));
    panel.enter().expect("test setup");

    assert!(panel.marked.is_empty());

    Ok(())
}

#[test]
fn test_panel_enter_file() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    File::create(temp_dir.path().join("test.txt")).expect("test setup");

    let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    let original_path = panel.path.clone();

    // Find and select the file
    for (i, entry) in panel.entries.iter().enumerate() {
        if entry.name == "test.txt" {
            panel.select(Some(i));
            break;
        }
    }

    // Entering a file should not change the path
    panel.enter().expect("test setup");
    assert_eq!(panel.path, original_path);

    Ok(())
}

#[test]
fn test_panel_enter_no_selection() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    panel.select(None);

    panel.enter().expect("test setup");
    assert_eq!(panel.path, temp_dir.path());

    Ok(())
}

#[test]
fn test_panel_enter_missing_entry() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    panel.select(Some(10_000));

    panel.enter().expect("test setup");
    assert_eq!(panel.path, temp_dir.path());

    Ok(())
}

#[test]
fn test_panel_get_selected_entry() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    File::create(temp_dir.path().join("test.txt")).expect("test setup");

    let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    // Should have an entry selected by default
    let entry = panel.get_selected_entry();
    assert!(entry.is_some());

    // Clear selection
    panel.select(None);
    assert!(panel.get_selected_entry().is_none());

    Ok(())
}

#[test]
fn test_panel_ensure_selection() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    File::create(temp_dir.path().join("test.txt")).expect("test setup");

    let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    // Clear selection
    panel.select(None);
    assert!(panel.selected().is_none());

    // Ensure selection should set it to 0
    panel.ensure_selection();
    assert_eq!(panel.selected(), Some(0));

    Ok(())
}

#[test]
fn test_panel_ensure_selection_empty() {
    let mut panel = Panel::from_entries(PathBuf::new(), Vec::new());

    // Should not set selection on empty panel
    panel.ensure_selection();
    assert!(panel.selected().is_none());
}

#[test]
fn test_panel_selected_path() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    File::create(temp_dir.path().join("test.txt")).expect("test setup");

    let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    // Find and select the file
    for (i, entry) in panel.entries.iter().enumerate() {
        if entry.name == "test.txt" {
            panel.select(Some(i));
            break;
        }
    }

    let selected_path = panel.selected_path();
    assert!(selected_path.is_some());
    assert_eq!(selected_path.unwrap(), temp_dir.path().join("test.txt"));

    // Clear selection
    panel.select(None);
    assert!(panel.selected_path().is_none());

    Ok(())
}

#[test]
fn test_panel_side_equality() {
    assert_eq!(PanelSide::Left, PanelSide::Left);
    assert_eq!(PanelSide::Right, PanelSide::Right);
    assert_ne!(PanelSide::Left, PanelSide::Right);
    assert_ne!(PanelSide::Right, PanelSide::Left);
}

#[test]
fn test_panel_side_copy() {
    let side = PanelSide::Left;
    let copied = side;
    assert_eq!(side, copied);
}

#[test]
fn test_app_new() -> io::Result<()> {
    let app = App::new().expect("test setup");

    assert_eq!(app.active_panel, PanelSide::Left);
    assert!(!app.is_overlay_active());
    assert_eq!(app.bookmarks.slots().len(), 10);
    assert!(app.lifecycle.is_running());
    assert_eq!(
        app.status_text(),
        "F1 Help  F3 View  F4 Edit  F5 Copy  F6 Move  F7 Mkdir  F8 Delete"
    );

    // Both panels should be in the current directory
    assert_eq!(app.left_panel.path, app.right_panel.path);

    Ok(())
}

#[test]
fn test_app_get_active_panel() -> io::Result<()> {
    let mut app = App::new().expect("test setup");

    // Initially active panel is left
    let left_path = app.left_panel.path.clone();
    let active_path = app.get_active_panel().path.clone();
    assert_eq!(active_path, left_path);

    // Switch to right panel
    app.active_panel = PanelSide::Right;
    let right_path = app.right_panel.path.clone();
    let active_path = app.get_active_panel().path.clone();
    assert_eq!(active_path, right_path);

    Ok(())
}

#[test]
fn test_app_panel_ref_getters() -> io::Result<()> {
    let temp_dir1 = TempDir::new().expect("test setup");
    let temp_dir2 = TempDir::new().expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir1.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir2.path().to_path_buf()).expect("test setup");

    app.active_panel = PanelSide::Left;
    assert_eq!(app.get_active_panel_ref().path, temp_dir1.path());
    assert_eq!(app.get_inactive_panel_ref().path, temp_dir2.path());

    app.active_panel = PanelSide::Right;
    assert_eq!(app.get_active_panel_ref().path, temp_dir2.path());
    assert_eq!(app.get_inactive_panel_ref().path, temp_dir1.path());

    Ok(())
}

#[test]
fn test_app_get_inactive_panel() -> io::Result<()> {
    let mut app = App::new().expect("test setup");

    // Initially active panel is left, so inactive is right
    let right_path = app.right_panel.path.clone();
    let inactive_path = app.get_inactive_panel().path.clone();
    assert_eq!(inactive_path, right_path);

    // Switch to right panel
    app.active_panel = PanelSide::Right;
    let left_path = app.left_panel.path.clone();
    let inactive_path = app.get_inactive_panel().path.clone();
    assert_eq!(inactive_path, left_path);

    Ok(())
}

#[test]
fn test_app_switch_panel() -> io::Result<()> {
    let mut app = App::new().expect("test setup");

    assert_eq!(app.active_panel, PanelSide::Left);

    app.switch_panel();
    assert_eq!(app.active_panel, PanelSide::Right);

    app.switch_panel();
    assert_eq!(app.active_panel, PanelSide::Left);

    Ok(())
}

#[test]
fn test_app_switch_panel_multiple_times() -> io::Result<()> {
    let mut app = App::new().expect("test setup");

    // Switch multiple times and ensure it alternates correctly
    for _ in 0..10 {
        let before = app.active_panel;
        app.switch_panel();
        let after = app.active_panel;

        match before {
            PanelSide::Left => assert_eq!(after, PanelSide::Right),
            PanelSide::Right => assert_eq!(after, PanelSide::Left),
        }
    }

    Ok(())
}
