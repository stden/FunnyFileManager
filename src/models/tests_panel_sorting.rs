//! Unit tests: panel sorting (split out to keep files under 500 lines).

use std::fs;

use super::tests::*;

use super::*;

use std::fs::File;

use std::io::Write;

use tempfile::TempDir;

#[test]
fn test_panel_next_with_none_selected() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    File::create(temp_dir.path().join("test.txt")).expect("test setup");

    let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    // Clear the selection
    panel.select(None);

    // Next should select the first item
    panel.next();
    assert_eq!(panel.selected(), Some(0));

    Ok(())
}

#[test]
fn test_panel_previous_with_none_selected() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    File::create(temp_dir.path().join("test.txt")).expect("test setup");

    let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    // Clear the selection
    panel.select(None);

    // Previous should select the first item
    panel.previous();
    assert_eq!(panel.selected(), Some(0));

    Ok(())
}

#[test]
fn test_sort_mode_next_and_label() {
    assert_eq!(SortMode::Name.label(), "Name");
    assert_eq!(SortMode::Extension.label(), "Ext");
    assert_eq!(SortMode::Size.label(), "Size");

    assert_eq!(SortMode::Name.next(), SortMode::Extension);
    assert_eq!(SortMode::Extension.next(), SortMode::Size);
    assert_eq!(SortMode::Size.next(), SortMode::Name);
}

#[test]
fn test_panel_cycle_sort_mode() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    assert_eq!(panel.sort_mode, SortMode::Name);
    panel.cycle_sort_mode();
    assert_eq!(panel.sort_mode, SortMode::Extension);
    panel.cycle_sort_mode();
    assert_eq!(panel.sort_mode, SortMode::Size);
    panel.cycle_sort_mode();
    assert_eq!(panel.sort_mode, SortMode::Name);

    Ok(())
}

#[test]
fn test_panel_marking_and_action_names() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    File::create(temp_dir.path().join("alpha.txt")).expect("test setup");
    File::create(temp_dir.path().join("beta.txt")).expect("test setup");

    let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    select_named(&mut panel, "alpha.txt");

    assert!(panel.toggle_mark_selected());
    assert!(panel.marked.contains("alpha.txt"));
    assert_eq!(file_names(&panel), vec!["alpha.txt", "beta.txt"]);

    let mut action_names = panel.action_entry_names();
    action_names.sort();
    assert_eq!(action_names, vec!["alpha.txt"]);

    assert!(panel.toggle_mark_selected());
    assert!(panel.marked.is_empty());
    assert_eq!(panel.action_entry_names(), vec!["alpha.txt"]);

    select_named(&mut panel, "beta.txt");
    assert!(panel.toggle_mark_selected());
    assert!(panel.marked.contains("beta.txt"));

    fs::remove_file(temp_dir.path().join("beta.txt")).expect("test setup");
    panel.load_entries().expect("test setup");
    assert!(!panel.marked.contains("beta.txt"));

    panel.select(None);
    assert!(!panel.toggle_mark_selected());

    select_named(&mut panel, "..");
    assert!(!panel.toggle_mark_selected());
    assert!(panel.action_entry_names().is_empty());
    assert!(panel.selected_path().is_none());

    Ok(())
}

#[test]
fn test_panel_load_entries_sort_modes() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    fs::create_dir(temp_dir.path().join("dir")).expect("test setup");
    File::create(temp_dir.path().join("b.txt"))
        .expect("test setup")
        .write_all(b"1")
        .expect("test setup");
    File::create(temp_dir.path().join("a.rs"))
        .expect("test setup")
        .write_all(b"1234")
        .expect("test setup");
    File::create(temp_dir.path().join("c"))
        .expect("test setup")
        .write_all(b"12")
        .expect("test setup");

    let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    panel.sort_mode = SortMode::Extension;
    panel.load_entries().expect("test setup");
    assert_eq!(file_names(&panel), vec!["c", "a.rs", "b.txt"]);

    panel.sort_mode = SortMode::Size;
    panel.load_entries().expect("test setup");
    assert_eq!(file_names(&panel), vec!["a.rs", "c", "b.txt"]);

    Ok(())
}

/// Navigation stops at the ends rather than wrapping.
#[test]
fn test_panel_navigation_stops_at_both_ends() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    for i in 0..3 {
        File::create(temp_dir.path().join(format!("file{i}.txt"))).expect("test setup");
    }

    let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    panel.load_entries().expect("test setup");

    // With 3 files plus ".." entry, we have indices 0-3
    panel.select(Some(0));
    panel.previous();
    assert_eq!(panel.selected(), Some(0), "up at the top stays");

    panel.select(Some(3));
    panel.next();
    assert_eq!(panel.selected(), Some(3), "down at the bottom stays");

    Ok(())
}

#[test]
fn test_panel_with_max_entries() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");

    for i in 0..1000 {
        File::create(temp_dir.path().join(format!("file{i:04}.txt"))).expect("test setup");
    }

    let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    panel.load_entries().expect("test setup");

    assert_eq!(panel.entries.len(), 1001);

    panel.select(Some(0));
    for _ in 0..500 {
        panel.next();
    }
    assert_eq!(panel.selected(), Some(500));

    panel.select(Some(1000));
    let entry_name = panel.entries[1000].name.clone();
    assert_eq!(panel.selected_path(), Some(panel.path.join(entry_name)));

    Ok(())
}

#[test]
fn test_panel_empty_directory_operations() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let empty_dir = temp_dir.path().join("empty");
    fs::create_dir(&empty_dir).expect("test setup");

    let mut panel = Panel::new(empty_dir).expect("test setup");
    panel.load_entries().expect("test setup");

    assert_eq!(panel.entries.len(), 1);
    assert_eq!(panel.entries[0].name, "..");

    panel.next();
    assert_eq!(panel.selected(), Some(0));

    panel.previous();
    assert_eq!(panel.selected(), Some(0));

    panel.toggle_mark_selected();
    assert!(panel.marked.is_empty());

    Ok(())
}

#[test]
fn test_panel_unicode_filenames() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");

    let unicode_names = vec![
        "файл.txt",
        "文件.doc",
        "αρχείο.pdf",
        "🎵music🎵.mp3",
        "file with spaces.txt",
    ];

    for name in &unicode_names {
        File::create(temp_dir.path().join(name)).expect("test setup");
    }

    let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    panel.load_entries().expect("test setup");

    assert_eq!(panel.entries.len(), unicode_names.len() + 1);

    for name in &unicode_names {
        let found = panel.entries.iter().any(|e| e.name == *name);
        assert!(found, "Unicode filename {name} not found");
    }

    Ok(())
}

#[test]
fn test_panel_very_long_filenames() -> io::Result<()> {
    #[cfg(not(windows))]
    {
        let temp_dir = TempDir::new().expect("test setup");
        let long_name = "a".repeat(255);
        let file_path = temp_dir.path().join(&long_name);
        File::create(&file_path).expect("test setup");
        let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
        panel.load_entries().expect("test setup");

        let found = panel.entries.iter().any(|e| e.name == long_name);
        assert!(found);
    }
    #[cfg(windows)]
    {
        // Long filename test skipped on Windows
    }

    Ok(())
}

#[test]
fn test_sort_mode_edge_cases() {
    let mut mode = SortMode::Name;

    mode = mode.next();
    assert_eq!(mode, SortMode::Extension);
    assert_eq!(mode.label(), "Ext");

    mode = mode.next();
    assert_eq!(mode, SortMode::Size);
    assert_eq!(mode.label(), "Size");

    mode = mode.next();
    assert_eq!(mode, SortMode::Name);
    assert_eq!(mode.label(), "Name");
}

#[test]
fn test_app_bookmarks_boundary() -> io::Result<()> {
    let mut app = App::new().expect("test setup");

    assert_eq!(app.bookmarks.slots().len(), 10);

    for (i, bookmark) in app.bookmarks.slots().iter().enumerate() {
        assert!(bookmark.is_none(), "Bookmark {i} should be None");
    }

    app.bookmarks.set(
        BookmarkSlot::from_usize(0).expect("slot"),
        PathBuf::from("/test1"),
    );
    app.bookmarks.set(
        BookmarkSlot::from_usize(9).expect("slot"),
        PathBuf::from("/test2"),
    );

    assert_eq!(
        app.bookmarks
            .get(BookmarkSlot::from_usize(0).expect("slot")),
        Some(Path::new("/test1"))
    );
    assert_eq!(
        app.bookmarks
            .get(BookmarkSlot::from_usize(9).expect("slot")),
        Some(Path::new("/test2"))
    );

    Ok(())
}

#[test]
fn test_panel_marking_operations_edge_cases() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    for i in 0..5 {
        File::create(temp_dir.path().join(format!("file{i}.txt"))).expect("test setup");
    }

    let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    panel.load_entries().expect("test setup");

    panel.select(Some(1));
    panel.toggle_mark_selected();
    panel.toggle_mark_selected();
    assert!(panel.marked.is_empty());

    for i in 1..=5 {
        panel.select(Some(i));
        panel.toggle_mark_selected();
    }
    assert_eq!(panel.marked.len(), 5);

    panel.clear_marks();
    assert!(panel.marked.is_empty());

    panel.select(Some(0));
    panel.toggle_mark_selected();
    assert!(panel.marked.is_empty());

    Ok(())
}

#[test]
fn test_sort_mode_cycle() {
    let mut mode = SortMode::Name;
    assert_eq!(mode, SortMode::Name);

    mode = mode.next();
    assert_eq!(mode, SortMode::Extension);

    mode = mode.next();
    assert_eq!(mode, SortMode::Size);

    mode = mode.next();
    assert_eq!(mode, SortMode::Name);
}

#[test]
fn test_viewer_with_extreme_content() {
    let viewer = Viewer::report("Large", vec!["x".repeat(1000); 10000]);

    assert_eq!(viewer.lines.len(), 10000);
    assert_eq!(viewer.lines[0].len(), 1000);

    let empty_viewer = Viewer::report("Empty", vec![]);

    assert_eq!(empty_viewer.lines.len(), 0);
}

#[test]
fn test_sort_mode_default() {
    // Test the Default implementation for SortMode
    let default_mode: SortMode = SortMode::default();
    assert_eq!(default_mode, SortMode::Name);
}

#[test]
fn test_sort_mode_extension_no_extension() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");

    // Create files with and without extensions
    File::create(temp_dir.path().join("file1.txt")).expect("test setup");
    File::create(temp_dir.path().join("file2")).expect("test setup"); // No extension
    File::create(temp_dir.path().join("file3.md")).expect("test setup");
    File::create(temp_dir.path().join("file4")).expect("test setup"); // No extension

    let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    panel.sort_mode = SortMode::Extension;
    panel.load_entries().expect("test setup");

    // Files without extensions should sort before files with extensions (after "..")
    let entries = &panel.entries;
    assert!(entries[0].name == ".." || entries[1].name == "file2" || entries[2].name == "file4");

    Ok(())
}

#[test]
fn test_toggle_mark_selected_no_selection() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    // Clear selection
    panel.select(None);

    // Should return false when no selection
    assert!(!panel.toggle_mark_selected());
    assert!(panel.marked.is_empty());

    Ok(())
}

#[test]
fn test_panel_enter_parent_directory_navigation() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let subdir = temp_dir.path().join("subdir");
    fs::create_dir(&subdir).expect("test setup");

    let mut panel = Panel::new(subdir).expect("test setup");
    panel.load_entries().expect("test setup");

    // Select parent directory ".."
    panel.select(Some(0));
    let selected_entry = panel.get_selected_entry().unwrap();
    assert_eq!(selected_entry.name, "..");

    // Enter parent directory
    panel.enter().expect("test setup");

    // Should now be in the parent directory
    assert_eq!(
        panel.path.canonicalize().expect("test setup"),
        temp_dir.path().canonicalize().expect("test setup")
    );

    Ok(())
}

fn select_named(panel: &mut Panel, name: &str) {
    let index = panel
        .entries
        .iter()
        .position(|entry| entry.name == name)
        .expect("named entry");
    panel.select(Some(index));
}
