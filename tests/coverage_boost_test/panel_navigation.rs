//! Purpose: Panel cursor movement, Enter and marking on empty panels and `..`.

use super::*;

// ═══════════════════════════════════════════════════════════════════
// Panel edge cases
// ═══════════════════════════════════════════════════════════════════

#[test]
fn test_panel_next_on_empty_does_nothing() {
    let mut panel = Panel::from_entries(PathBuf::from("/nonexistent"), Vec::new());
    panel.next();
    assert_eq!(panel.selected(), None);
}

#[test]
fn test_panel_previous_on_empty_does_nothing() {
    let mut panel = Panel::from_entries(PathBuf::from("/nonexistent"), Vec::new());
    panel.previous();
    assert_eq!(panel.selected(), None);
}

#[test]
fn test_panel_enter_with_no_selection() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut panel = Panel::new(temp.path().to_path_buf())?;
    panel.select(None);
    panel.enter()?; // Should not panic
    Ok(())
}

#[test]
fn test_panel_enter_on_file_does_nothing() -> io::Result<()> {
    let temp = TempDir::new()?;
    fs::write(temp.path().join("regular.txt"), "data")?;
    let mut panel = Panel::new(temp.path().to_path_buf())?;
    let idx = panel
        .entries
        .iter()
        .position(|e| e.name == "regular.txt")
        .unwrap();
    panel.select(Some(idx));
    let path_before = panel.path.clone();
    panel.enter()?;
    assert_eq!(panel.path, path_before);
    Ok(())
}

#[test]
fn test_panel_toggle_mark_on_parent_returns_false() -> io::Result<()> {
    let temp = TempDir::new()?;
    fs::write(temp.path().join("f.txt"), "x")?;
    let mut panel = Panel::new(temp.path().to_path_buf())?;
    panel.select(Some(0)); // ".."
    assert!(!panel.toggle_mark_selected());
    assert_eq!(panel.marked.len(), 0);
    Ok(())
}

#[test]
fn test_panel_toggle_mark_with_no_selection_returns_false() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut panel = Panel::new(temp.path().to_path_buf())?;
    panel.select(None);
    assert!(!panel.toggle_mark_selected());
    Ok(())
}

#[test]
fn test_panel_enter_parent_selects_prev_dir() -> io::Result<()> {
    let temp = TempDir::new()?;
    let sub = temp.path().join("child");
    fs::create_dir(&sub)?;
    let mut panel = Panel::new(sub)?;
    panel.select(Some(0)); // ".."
    panel.enter()?;
    // After going up, "child" should be selected
    let sel = panel.get_selected_entry().unwrap();
    assert_eq!(sel.name, "child");
    Ok(())
}
