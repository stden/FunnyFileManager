//! Purpose: unit tests for the selected entry, keeping a selection, and the
//! selected path.

use super::*;

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
