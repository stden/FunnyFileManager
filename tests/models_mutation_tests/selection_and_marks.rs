//! Purpose: Marks, action targets, the selected path and quick-find matching.

use super::*;

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
