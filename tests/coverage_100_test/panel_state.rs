//! Purpose: Sort modes, selection, action targets, entry creation and navigation limits.

use super::*;

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
    assert!(names.contains(&"file1.txt".to_owned()));
    assert!(names.contains(&"file3.txt".to_owned()));

    // Clear marks and test with selection only
    panel.marked.clear();
    let names = panel.action_entry_names();
    assert_eq!(names.len(), 1);
    assert!(names.contains(&"file2.txt".to_owned()));

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
fn test_dir_entry_creation() {
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
}

#[test]
fn test_panel_navigation_stops_at_both_ends() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    for i in 1..=3 {
        fs::write(temp_dir.path().join(format!("file{i}.txt")), b"test")?;
    }

    let mut panel = Panel::new(temp_dir.path().to_path_buf())?;
    let last = panel.entries.len() - 1;

    // Down at the bottom stays at the bottom, as Far does.
    panel.select(Some(last));
    panel.next();
    assert_eq!(panel.selected(), Some(last));

    // And Up at the top stays at the top.
    panel.select(Some(0));
    panel.previous();
    assert_eq!(panel.selected(), Some(0));

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
