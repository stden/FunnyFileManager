//! Purpose: Marks, the selected path and action targets around `..` and empty panels.

use super::*;

// ---------------------------------------------------------------------------
// 4. Panel::clear_marks — verify marks are actually cleared
// ---------------------------------------------------------------------------

#[test]
fn test_panel_clear_marks_empties_set() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    File::create(temp_dir.path().join("a.txt"))?;
    File::create(temp_dir.path().join("b.txt"))?;

    let mut panel = Panel::new(temp_dir.path().to_path_buf())?;
    panel.marked.insert(FileName::from("a.txt"));
    panel.marked.insert(FileName::from("b.txt"));
    assert_eq!(panel.marked.len(), 2);

    panel.clear_marks();
    assert_eq!(panel.marked.len(), 0);

    Ok(())
}

// ---------------------------------------------------------------------------
// 10. Panel::toggle_mark_selected on ".." entry returns false
// ---------------------------------------------------------------------------

#[test]
fn test_toggle_mark_dotdot_returns_false() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let sub = temp_dir.path().join("sub");
    fs::create_dir(&sub)?;

    let mut panel = Panel::new(sub)?;
    // First entry should be ".."
    panel.select(Some(0));
    assert_eq!(panel.entries[0].name, "..");
    assert!(!panel.toggle_mark_selected());
    assert_eq!(panel.marked.len(), 0);

    Ok(())
}

// ---------------------------------------------------------------------------
// 11. Panel::selected_path returns None for ".." entry
// ---------------------------------------------------------------------------

#[test]
fn test_selected_path_returns_none_for_dotdot() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let sub = temp_dir.path().join("sub");
    fs::create_dir(&sub)?;

    let panel = Panel::new(sub)?;
    // ".." should give None for selected_path
    assert_eq!(panel.entries[0].name, "..");
    // state selects 0 by default
    assert!(panel.selected_path().is_none());

    // But a real file should give Some
    Ok(())
}

// ---------------------------------------------------------------------------
// 12. Panel::action_entry_names returns empty when ".." is selected and no marks
// ---------------------------------------------------------------------------

#[test]
fn test_action_entry_names_dotdot_no_marks_returns_empty() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let sub = temp_dir.path().join("sub");
    fs::create_dir(&sub)?;
    File::create(sub.join("file.txt"))?;

    let mut panel = Panel::new(sub)?;
    panel.select(Some(0)); // ".."
    let names = panel.action_entry_names();
    assert_eq!(names.len(), 0);

    Ok(())
}

// ---------------------------------------------------------------------------
// 13. Panel::action_entry_names returns marked names when marks exist
// ---------------------------------------------------------------------------

#[test]
fn test_action_entry_names_returns_marks_over_selection() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    File::create(temp_dir.path().join("a.txt"))?;
    File::create(temp_dir.path().join("b.txt"))?;

    let mut panel = Panel::new(temp_dir.path().to_path_buf())?;
    panel.marked.insert(FileName::from("a.txt"));
    panel.marked.insert(FileName::from("b.txt"));
    // Even though something else might be selected, marks take precedence
    let names = panel.action_entry_names();
    assert_eq!(names.len(), 2);
    assert!(names.contains(&"a.txt".to_owned()));
    assert!(names.contains(&"b.txt".to_owned()));

    Ok(())
}

// ---------------------------------------------------------------------------
// 14. Panel navigation on empty panel is a no-op
// ---------------------------------------------------------------------------

#[test]
fn test_panel_next_previous_on_empty_panel() {
    let mut panel = Panel::from_entries(PathBuf::from("/nonexistent"), vec![]);
    // Should not panic
    panel.next();
    panel.previous();
    assert!(panel.selected().is_none());
}

// ---------------------------------------------------------------------------
// 15. Panel::ensure_selection on non-empty panel with no selection
// ---------------------------------------------------------------------------

#[test]
fn test_ensure_selection_selects_first_when_none() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    File::create(temp_dir.path().join("file.txt"))?;

    let mut panel = Panel::new(temp_dir.path().to_path_buf())?;
    panel.select(None);
    panel.ensure_selection();
    assert_eq!(panel.selected(), Some(0));

    Ok(())
}

// ---------------------------------------------------------------------------
// 16. Panel::load_entries prunes stale marks
// ---------------------------------------------------------------------------

#[test]
fn test_load_entries_prunes_stale_marks() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    File::create(temp_dir.path().join("keep.txt"))?;
    File::create(temp_dir.path().join("remove_me.txt"))?;

    let mut panel = Panel::new(temp_dir.path().to_path_buf())?;
    panel.marked.insert(FileName::from("keep.txt"));
    panel.marked.insert(FileName::from("remove_me.txt"));
    panel.marked.insert(FileName::from("already_gone.txt"));

    // Delete one file
    fs::remove_file(temp_dir.path().join("remove_me.txt"))?;
    panel.load_entries()?;

    // "already_gone.txt" and "remove_me.txt" should be pruned
    assert!(panel.marked.contains("keep.txt"));
    assert!(!panel.marked.contains("remove_me.txt"));
    assert!(!panel.marked.contains("already_gone.txt"));

    Ok(())
}
