//! Purpose: Panels on a missing path, sort mode methods and selection after a reload.

use super::*;

#[test]
fn test_panel_new_invalid_path() {
    let result = Panel::new(PathBuf::from("/non/existent/path/that/should/fail"));
    assert!(result.is_err());
}

#[test]
fn test_sort_mode_methods() {
    let mode = SortMode::Name;
    assert_eq!(mode.label(), "Name");
    let next = mode.next();
    assert_eq!(next, SortMode::Extension);
    assert_eq!(next.label(), "Ext");
    let next2 = next.next();
    assert_eq!(next2, SortMode::Size);
    assert_eq!(next2.label(), "Size");
    let next3 = next2.next();
    assert_eq!(next3, SortMode::Name);
}

#[test]
fn test_panel_select_after_reload_gone() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    File::create(temp_dir.path().join("a.txt"))?;
    File::create(temp_dir.path().join("b.txt"))?;

    let mut panel = Panel::new(temp_dir.path().to_path_buf())?;
    // Select b.txt
    let b_index = panel
        .entries
        .iter()
        .position(|e| e.name == "b.txt")
        .unwrap();
    panel.select(Some(b_index));

    // Delete b.txt and reload
    fs::remove_file(temp_dir.path().join("b.txt"))?;
    panel.load_entries()?;

    // Should fallback to first entry (or .. if present)
    assert!(panel.selected().is_some());
    assert_ne!(panel.entries[panel.selected().unwrap()].name, "b.txt");

    Ok(())
}
