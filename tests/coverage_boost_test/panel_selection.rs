//! Purpose: Selected paths, action targets, marks after reload and quick-find matching.

use super::*;

#[test]
fn test_panel_selected_path_for_parent_returns_none() -> io::Result<()> {
    let temp = TempDir::new()?;
    fs::write(temp.path().join("f.txt"), "x")?;
    let mut panel = Panel::new(temp.path().to_path_buf())?;
    panel.select(Some(0)); // ".."
    assert!(panel.selected_path().is_none());
    Ok(())
}

#[test]
fn test_panel_action_entry_names_with_marks() -> io::Result<()> {
    let temp = TempDir::new()?;
    fs::write(temp.path().join("a.txt"), "a")?;
    fs::write(temp.path().join("b.txt"), "b")?;
    let mut panel = Panel::new(temp.path().to_path_buf())?;
    panel.marked.insert(FileName::from("a.txt"));
    panel.marked.insert(FileName::from("b.txt"));
    let names = panel.action_entry_names();
    assert_eq!(names.len(), 2);
    assert!(names.contains(&"a.txt".to_owned()));
    Ok(())
}

#[test]
fn test_panel_action_entry_names_without_marks() -> io::Result<()> {
    let temp = TempDir::new()?;
    fs::write(temp.path().join("sel.txt"), "s")?;
    let mut panel = Panel::new(temp.path().to_path_buf())?;
    let idx = panel
        .entries
        .iter()
        .position(|e| e.name == "sel.txt")
        .unwrap();
    panel.select(Some(idx));
    let names = panel.action_entry_names();
    assert_eq!(names, vec!["sel.txt"]);
    Ok(())
}

#[test]
fn test_panel_action_entry_names_on_parent() -> io::Result<()> {
    let temp = TempDir::new()?;
    fs::write(temp.path().join("x.txt"), "x")?;
    let mut panel = Panel::new(temp.path().to_path_buf())?;
    panel.select(Some(0)); // ".."
    let names = panel.action_entry_names();
    assert!(names.is_empty());
    Ok(())
}

#[test]
fn test_panel_cycle_sort_mode_full_cycle() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut panel = Panel::new(temp.path().to_path_buf())?;
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
fn test_panel_marks_cleaned_on_reload() -> io::Result<()> {
    let temp = TempDir::new()?;
    fs::write(temp.path().join("exists.txt"), "y")?;
    let mut panel = Panel::new(temp.path().to_path_buf())?;
    panel.marked.insert(FileName::from("exists.txt"));
    panel.marked.insert(FileName::from("gone.txt")); // doesn't exist
    panel.load_entries()?;
    assert!(panel.marked.contains("exists.txt"));
    assert!(!panel.marked.contains("gone.txt")); // cleaned up
    Ok(())
}

#[test]
fn test_panel_find_next_matching_wraps() -> io::Result<()> {
    let temp = TempDir::new()?;
    fs::write(temp.path().join("alpha.txt"), "a")?;
    fs::write(temp.path().join("beta.txt"), "b")?;
    fs::write(temp.path().join("another.txt"), "a2")?;
    let mut panel = Panel::new(temp.path().to_path_buf())?;
    // Find first "a" match
    assert!(panel.find_first_matching("a"));
    let first = panel.selected().unwrap();
    // Find next "a" match
    assert!(panel.find_next_matching("a"));
    let second = panel.selected().unwrap();
    assert_ne!(first, second);
    Ok(())
}

#[test]
fn test_panel_find_matching_empty_prefix() -> io::Result<()> {
    let temp = TempDir::new()?;
    fs::write(temp.path().join("a.txt"), "a")?;
    let mut panel = Panel::new(temp.path().to_path_buf())?;
    assert!(!panel.find_first_matching(""));
    assert!(!panel.find_next_matching(""));
    Ok(())
}

#[test]
fn test_panel_ensure_selection_on_nonempty() -> io::Result<()> {
    let temp = TempDir::new()?;
    fs::write(temp.path().join("a.txt"), "a")?;
    let mut panel = Panel::new(temp.path().to_path_buf())?;
    panel.select(None);
    panel.ensure_selection();
    assert_eq!(panel.selected(), Some(0));
    Ok(())
}
