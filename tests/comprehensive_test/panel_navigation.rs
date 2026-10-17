//! Purpose: Cursor movement, Enter and marking inside one panel.

use super::*;

// ==================== Panel Navigation Edge Cases ====================

#[test]
fn test_panel_next_stops_at_the_last_entry() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("a.txt"), b"").unwrap();
    fs::write(temp.path().join("b.txt"), b"").unwrap();

    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    // entries: ["..", "a.txt", "b.txt"]
    let last = panel.entries.len() - 1;
    panel.select(Some(last));
    panel.next();
    assert_eq!(
        panel.selected(),
        Some(last),
        "Down at the bottom stays there rather than jumping to the top"
    );
}

#[test]
fn test_panel_previous_stops_at_the_first_entry() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("a.txt"), b"").unwrap();

    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    panel.select(Some(0));
    panel.previous();
    assert_eq!(
        panel.selected(),
        Some(0),
        "Up at the top stays there rather than jumping to the bottom",
    );
}

#[test]
fn test_panel_next_empty_panel() {
    let mut panel = Panel::from_entries(PathBuf::from("/nonexistent_test_path"), vec![]);
    panel.next(); // should not panic
    assert_eq!(panel.selected(), None);
}

#[test]
fn test_panel_previous_empty_panel() {
    let mut panel = Panel::from_entries(PathBuf::from("/nonexistent_test_path"), vec![]);
    panel.previous(); // should not panic
    assert_eq!(panel.selected(), None);
}

#[test]
fn test_panel_next_no_selection() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("a.txt"), b"").unwrap();

    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    panel.select(None);
    panel.next();
    assert_eq!(panel.selected(), Some(0), "should select first item");
}

#[test]
fn test_panel_previous_no_selection() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("a.txt"), b"").unwrap();

    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    panel.select(None);
    panel.previous();
    assert_eq!(panel.selected(), Some(0), "should select first item");
}

#[test]
fn test_panel_single_entry() {
    let temp = TempDir::new().unwrap();
    // Only ".." entry since path has parent
    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    assert!(!panel.entries.is_empty());
    panel.select(Some(0));
    panel.next();
    // With one entry, wraps back to 0
    assert_eq!(panel.selected(), Some(0));
}

// ==================== Enter on File vs Directory ====================

#[test]
fn test_panel_enter_on_file_does_nothing() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("file.txt"), b"data").unwrap();

    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    let file_idx = panel
        .entries
        .iter()
        .position(|e| e.name == "file.txt")
        .unwrap();
    panel.select(Some(file_idx));

    let path_before = panel.path.clone();
    panel.enter().unwrap();
    assert_eq!(panel.path, path_before, "path should not change for files");
}

#[test]
fn test_panel_enter_on_dotdot() {
    let temp = TempDir::new().unwrap();
    let sub = temp.path().join("sub");
    fs::create_dir(&sub).unwrap();

    let mut panel = Panel::new(sub).unwrap();
    // ".." is at index 0
    panel.select(Some(0));
    panel.enter().unwrap();
    // Should go to parent
    assert_eq!(panel.path, temp.path().to_path_buf());
}

#[test]
fn test_panel_enter_no_selection() {
    let temp = TempDir::new().unwrap();
    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    panel.select(None);
    // Should not panic
    panel.enter().unwrap();
}

// ==================== Mark/Unmark ====================

#[test]
fn test_toggle_mark_dotdot_returns_false() {
    let temp = TempDir::new().unwrap();
    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    panel.select(Some(0)); // ".."
    let result = panel.toggle_mark_selected();
    assert!(!result, "marking '..' should return false");
    assert!(panel.marked.is_empty());
}

#[test]
fn test_toggle_mark_no_selection() {
    let temp = TempDir::new().unwrap();
    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    panel.select(None);
    let result = panel.toggle_mark_selected();
    assert!(!result, "marking with no selection should return false");
}

#[test]
fn test_marks_cleared_on_enter_directory() {
    let temp = TempDir::new().unwrap();
    let sub = temp.path().join("sub");
    fs::create_dir(&sub).unwrap();
    fs::write(temp.path().join("file.txt"), b"").unwrap();

    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    // Mark a file
    let file_idx = panel
        .entries
        .iter()
        .position(|e| e.name == "file.txt")
        .unwrap();
    panel.select(Some(file_idx));
    panel.toggle_mark_selected();
    assert!(!panel.marked.is_empty());

    // Enter subdirectory
    let sub_idx = panel.entries.iter().position(|e| e.name == "sub").unwrap();
    panel.select(Some(sub_idx));
    panel.enter().unwrap();
    assert!(panel.marked.is_empty(), "marks should clear on dir change");
}
