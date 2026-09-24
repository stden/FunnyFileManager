//! Purpose: Panel cursor movement, empty directories, selection repair and Enter.

use super::*;

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
