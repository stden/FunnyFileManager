//! Purpose: Strengthens model assertions against mutation-test survivors.
//!
use funny_file_manager::models::EntryKind;
use funny_file_manager::models::FileName;
use funny_file_manager::models::FileSize;
use funny_file_manager::models::Modified;
use funny_file_manager::models::{App, DirEntry, Panel, PanelSide, SortMode, Viewer};
use funny_file_manager::scroll::ScrollTop;
use std::fs::{self, File};
use std::io::{self, Write};
use tempfile::TempDir;

#[expect(
    clippy::expect_used,
    reason = "test fixture: a panel without the named entry fails the test that chose it"
)]
fn select_entry(panel: &mut Panel, name: &str) {
    let index = panel
        .entries
        .iter()
        .position(|entry| entry.name == name)
        .expect("entry not found");
    panel.select(Some(index));
}

#[test]
fn test_panel_navigation_mutations() -> io::Result<()> {
    let temp_dir = TempDir::new()?;

    // Create test files
    for i in 1..=5 {
        File::create(temp_dir.path().join(format!("file{i}.txt")))?;
    }

    let mut panel = Panel::new(temp_dir.path().to_path_buf())?;

    // Navigation stops at the ends rather than wrapping: holding Down to reach
    // the bottom of a long listing should end there, not put the cursor back on
    // ".." where one more keypress leaves the directory.
    let last = panel.entries.len() - 1;
    panel.select(Some(last));
    panel.next();
    assert_eq!(panel.selected(), Some(last), "down at the bottom stays");

    panel.select(Some(0));
    panel.previous();
    assert_eq!(panel.selected(), Some(0), "up at the top stays");

    // Test next() incrementing
    panel.select(Some(1));
    panel.next();
    assert_eq!(panel.selected(), Some(2), "Should increment");

    // Test previous() decrementing
    panel.select(Some(2));
    panel.previous();
    assert_eq!(panel.selected(), Some(1), "Should decrement");

    Ok(())
}

#[test]
fn test_panel_empty_directory_navigation() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut panel = Panel::new(temp_dir.path().to_path_buf())?;

    // Test navigation in empty directory
    panel.entries.clear();
    panel.select(None);

    panel.next();
    assert_eq!(panel.selected(), None, "Should remain None in empty dir");

    panel.previous();
    assert_eq!(panel.selected(), None, "Should remain None in empty dir");

    Ok(())
}

#[test]
fn test_panel_ensure_selection_mutations() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    File::create(temp_dir.path().join("test.txt"))?;

    let mut panel = Panel::new(temp_dir.path().to_path_buf())?;

    // Test ensure_selection when None
    panel.select(None);
    panel.ensure_selection();
    assert_eq!(panel.selected(), Some(0), "Should select first when None");

    // Test ensure_selection when already selected
    panel.select(Some(0));
    panel.ensure_selection();
    assert_eq!(panel.selected(), Some(0), "Should keep selection");

    // Test with empty entries
    panel.entries.clear();
    panel.select(None);
    panel.ensure_selection();
    assert_eq!(panel.selected(), None, "Should remain None when empty");

    Ok(())
}

#[test]
fn test_sort_mode_mutations() {
    // Test next() cycling
    let mode = SortMode::Name;
    assert!(matches!(mode.next(), SortMode::Extension));
    assert!(matches!(mode.next().next(), SortMode::Size));
    assert!(matches!(mode.next().next().next(), SortMode::Name));

    // Test label() returning correct strings
    assert_eq!(SortMode::Name.label(), "Name");
    assert_eq!(SortMode::Extension.label(), "Ext");
    assert_eq!(SortMode::Size.label(), "Size");
}

#[test]
fn test_panel_sort_mutations() -> io::Result<()> {
    let temp_dir = TempDir::new()?;

    // Create files with different extensions and sizes
    File::create(temp_dir.path().join("b.txt"))?.write_all(b"small")?;
    File::create(temp_dir.path().join("a.rs"))?.write_all(b"medium file")?;
    File::create(temp_dir.path().join("c.md"))?.write_all(b"very large file content")?;

    let mut panel = Panel::new(temp_dir.path().to_path_buf())?;

    // Test sorting by name
    panel.sort_mode = SortMode::Name;
    panel.load_entries()?;
    assert_eq!(panel.entries[0].name, "..");
    assert_eq!(panel.entries[1].name, "a.rs");
    assert_eq!(panel.entries[2].name, "b.txt");
    assert_eq!(panel.entries[3].name, "c.md");

    // Test sorting by extension
    panel.sort_mode = SortMode::Extension;
    panel.load_entries()?;
    assert_eq!(panel.entries[0].name, "..");
    for (entry, extension) in panel.entries[1..].iter().zip(["md", "rs", "txt"]) {
        assert!(entry
            .name
            .rsplit_once('.')
            .is_some_and(|(_, suffix)| suffix.eq_ignore_ascii_case(extension)));
    }

    // Test sorting by size
    panel.sort_mode = SortMode::Size;
    panel.load_entries()?;
    assert_eq!(panel.entries[0].name, "..");
    // Files should be sorted by size
    let sizes: Vec<u64> = panel.entries[1..].iter().map(|e| e.size.bytes()).collect();
    for i in 1..sizes.len() {
        assert!(sizes[i] <= sizes[i - 1], "Should be sorted by size");
    }

    Ok(())
}

#[test]
fn test_panel_marking_mutations() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    File::create(temp_dir.path().join("file1.txt"))?;
    File::create(temp_dir.path().join("file2.txt"))?;

    let mut panel = Panel::new(temp_dir.path().to_path_buf())?;

    // Test toggle_mark with selection
    select_entry(&mut panel, "file1.txt");
    panel.toggle_mark_selected();
    assert!(panel.marked.contains("file1.txt"));

    // Toggle again to unmark
    panel.toggle_mark_selected();
    assert!(!panel.marked.contains("file1.txt"));

    // Test toggle_mark with no selection
    panel.select(None);
    panel.toggle_mark_selected();
    assert!(panel.marked.is_empty());

    // Test clear_marks
    panel.marked.insert(FileName::from("file1.txt"));
    panel.marked.insert(FileName::from("file2.txt"));
    assert_eq!(panel.marked.len(), 2);
    panel.clear_marks();
    assert!(panel.marked.is_empty());

    Ok(())
}

#[test]
fn test_panel_action_entry_names_mutations() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    File::create(temp_dir.path().join("file1.txt"))?;
    File::create(temp_dir.path().join("file2.txt"))?;
    File::create(temp_dir.path().join("file3.txt"))?;

    let mut panel = Panel::new(temp_dir.path().to_path_buf())?;

    // Test with marks
    panel.marked.insert(FileName::from("file1.txt"));
    panel.marked.insert(FileName::from("file3.txt"));
    select_entry(&mut panel, "file2.txt");

    let names = panel.action_entry_names();
    assert_eq!(names.len(), 2);
    assert!(names.contains(&"file1.txt".to_owned()));
    assert!(names.contains(&"file3.txt".to_owned()));

    // Test without marks (should return selected)
    panel.marked.clear();
    select_entry(&mut panel, "file2.txt");
    let names = panel.action_entry_names();
    assert_eq!(names.len(), 1);
    assert!(names.contains(&"file2.txt".to_owned()));

    // Test with no selection and no marks
    panel.select(None);
    let names = panel.action_entry_names();
    assert!(names.is_empty());

    // Test parent directory ".."
    select_entry(&mut panel, "..");
    let names = panel.action_entry_names();
    assert!(names.is_empty(), "Parent directory should not be included");

    Ok(())
}

#[test]
fn test_app_panel_getters_mutations() -> io::Result<()> {
    let mut app = App::new()?;

    // Test get_active_panel with Left
    app.active_panel = PanelSide::Left;
    {
        let active = app.get_active_panel();
        active.sort_mode = SortMode::Size;
    }
    assert_eq!(app.left_panel.sort_mode, SortMode::Size);
    assert_eq!(app.right_panel.sort_mode, SortMode::Name);

    // Test get_active_panel with Right
    app.active_panel = PanelSide::Right;
    {
        let active = app.get_active_panel();
        active.sort_mode = SortMode::Extension;
    }
    assert_eq!(app.right_panel.sort_mode, SortMode::Extension);

    // Test get_inactive_panel with Left active
    app.active_panel = PanelSide::Left;
    {
        let inactive = app.get_inactive_panel();
        inactive.sort_mode = SortMode::Size;
    }
    assert_eq!(app.right_panel.sort_mode, SortMode::Size);

    // Test get_inactive_panel with Right active
    app.active_panel = PanelSide::Right;
    {
        let inactive = app.get_inactive_panel();
        inactive.sort_mode = SortMode::Name;
    }
    assert_eq!(app.left_panel.sort_mode, SortMode::Name);

    Ok(())
}

#[test]
fn test_app_switch_panel_mutations() -> io::Result<()> {
    let mut app = App::new()?;

    // Test switching from Left to Right
    app.active_panel = PanelSide::Left;
    app.switch_panel();
    assert!(matches!(app.active_panel, PanelSide::Right));

    // Test switching from Right to Left
    app.switch_panel();
    assert!(matches!(app.active_panel, PanelSide::Left));

    Ok(())
}

#[test]
fn test_viewer_mutations() {
    // Test creation
    let viewer = Viewer {
        scroll: ScrollTop::TOP.down(5, usize::MAX),
        ..Viewer::report("test.txt", vec!["Line 1".to_owned(), "Line 2".to_owned()])
    };

    assert_eq!(viewer.title(), "test.txt");
    assert_eq!(viewer.lines.len(), 2);
    assert_eq!(viewer.scroll.lines(), 5);

    // Test default
    let default_viewer = Viewer::default();
    assert!(default_viewer.title().is_empty());
    assert!(default_viewer.lines.is_empty());
    assert_eq!(default_viewer.scroll, ScrollTop::TOP);
}

#[test]
fn test_dir_entry_mutations() {
    // Test creation
    let entry = DirEntry {
        name: "test.txt".to_owned(),
        kind: EntryKind::File,
        size: FileSize::from_bytes(1024),
        modified: Modified::UNKNOWN,
        executable: false,
        ..DirEntry::default()
    };

    assert_eq!(entry.name, "test.txt");
    assert!(!entry.kind.is_dir());
    assert_eq!(entry.size, FileSize::from_bytes(1024));

    // Test clone
    let cloned = entry.clone();
    assert_eq!(cloned.name, entry.name);
    assert_eq!(cloned.kind.is_dir(), entry.kind.is_dir());
    assert_eq!(cloned.size, entry.size);
}

#[test]
fn test_panel_enter_mutations() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let subdir = temp_dir.path().join("subdir");
    fs::create_dir(&subdir)?;
    File::create(temp_dir.path().join("file.txt"))?;

    let mut panel = Panel::new(temp_dir.path().to_path_buf())?;
    let original_path = panel.path.clone();

    // Test entering a file (should not change path)
    select_entry(&mut panel, "file.txt");
    panel.enter()?;
    assert_eq!(
        panel.path, original_path,
        "Path should not change for files"
    );

    // Test entering a directory
    select_entry(&mut panel, "subdir");
    panel.enter()?;
    assert!(panel.path.ends_with("subdir"), "Should enter subdirectory");
    assert!(panel.marked.is_empty(), "Marks should be cleared");

    // Test entering parent directory
    select_entry(&mut panel, "..");
    panel.enter()?;
    assert_eq!(panel.path, original_path, "Should return to parent");

    Ok(())
}

#[test]
fn test_panel_selected_path_mutations() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    File::create(temp_dir.path().join("test.txt"))?;

    let mut panel = Panel::new(temp_dir.path().to_path_buf())?;

    // Test with selection
    select_entry(&mut panel, "test.txt");
    let path = panel.selected_path();
    assert!(path.is_some());
    assert!(path.unwrap().ends_with("test.txt"));

    // Test with no selection
    panel.select(None);
    let path = panel.selected_path();
    assert!(path.is_none());

    // Test with empty entries
    panel.entries.clear();
    panel.select(Some(0));
    let path = panel.selected_path();
    assert!(path.is_none());

    Ok(())
}

#[test]
fn test_panel_find_first_matching_full_coverage() -> io::Result<()> {
    let temp_dir = TempDir::new()?;

    // Create test files
    File::create(temp_dir.path().join("aaa.txt"))?;
    File::create(temp_dir.path().join("bbb.txt"))?;
    File::create(temp_dir.path().join("abc.txt"))?;

    let mut panel = Panel::new(temp_dir.path().to_path_buf())?;

    // Test 1: No current selection - should find first match
    panel.select(None);
    assert!(panel.find_first_matching("b"));
    assert_eq!(panel.get_selected_entry().unwrap().name, "bbb.txt");

    // Test 2: Current selection matches pattern - should return first match
    select_entry(&mut panel, "aaa.txt");
    assert!(panel.find_first_matching("a"));
    assert_eq!(panel.get_selected_entry().unwrap().name, "aaa.txt");

    // Test 3: No match found
    assert!(!panel.find_first_matching("xyz"));

    // Test 4: Pattern matches only current selection
    select_entry(&mut panel, "bbb.txt");
    assert!(panel.find_first_matching("bbb"));
    // Should stay on bbb.txt since it's the only match
    assert_eq!(panel.get_selected_entry().unwrap().name, "bbb.txt");

    Ok(())
}
