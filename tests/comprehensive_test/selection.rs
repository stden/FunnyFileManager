//! Purpose: Selected path, action targets and restoring the selection after a reload.

use super::*;

// ==================== selected_path ====================

#[test]
fn test_selected_path_on_dotdot_returns_none() {
    let temp = TempDir::new().unwrap();
    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    panel.select(Some(0)); // ".."
    assert!(panel.selected_path().is_none());
}

#[test]
fn test_selected_path_on_file() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("x.txt"), b"").unwrap();
    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    let idx = panel
        .entries
        .iter()
        .position(|e| e.name == "x.txt")
        .unwrap();
    panel.select(Some(idx));
    let p = panel.selected_path().unwrap();
    assert_eq!(p, temp.path().join("x.txt"));
}

// ==================== action_entry_names ====================

#[test]
fn test_action_entry_names_with_marks() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("a.txt"), b"").unwrap();
    fs::write(temp.path().join("b.txt"), b"").unwrap();

    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    panel.marked.insert(FileName::from("a.txt"));
    panel.marked.insert(FileName::from("b.txt"));
    let names = panel.action_entry_names();
    assert!(names.contains(&"a.txt".to_owned()));
    assert!(names.contains(&"b.txt".to_owned()));
}

#[test]
fn test_action_entry_names_selected_file() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("a.txt"), b"").unwrap();

    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    let idx = panel
        .entries
        .iter()
        .position(|e| e.name == "a.txt")
        .unwrap();
    panel.select(Some(idx));
    let names = panel.action_entry_names();
    assert_eq!(names, vec!["a.txt"]);
}

#[test]
fn test_action_entry_names_dotdot_returns_empty() {
    let temp = TempDir::new().unwrap();
    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    panel.select(Some(0)); // ".."
    let names = panel.action_entry_names();
    assert!(names.is_empty());
}

// ==================== ensure_selection ====================

#[test]
fn test_ensure_selection_on_empty_panel() {
    let mut panel = Panel::from_entries(PathBuf::from("/tmp"), vec![]);
    panel.select(None);
    panel.ensure_selection();
    assert_eq!(panel.selected(), None, "empty panel stays None");
}

#[test]
fn test_ensure_selection_with_entries() {
    let temp = TempDir::new().unwrap();
    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    panel.select(None);
    panel.ensure_selection();
    assert_eq!(panel.selected(), Some(0));
}

// ==================== select_after_reload ====================

#[test]
fn test_select_after_reload_restores_position() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("keep.txt"), b"").unwrap();
    fs::write(temp.path().join("other.txt"), b"").unwrap();

    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    let idx = panel
        .entries
        .iter()
        .position(|e| e.name == "keep.txt")
        .unwrap();
    panel.select(Some(idx));

    // Reload should restore selection to "keep.txt"
    panel.load_entries().unwrap();
    let selected = panel.get_selected_entry().unwrap();
    assert_eq!(selected.name, "keep.txt");
}

#[test]
fn test_select_after_reload_deleted_entry_falls_back() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("gone.txt"), b"").unwrap();

    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    let idx = panel
        .entries
        .iter()
        .position(|e| e.name == "gone.txt")
        .unwrap();
    panel.select(Some(idx));

    // Delete the file and reload
    fs::remove_file(temp.path().join("gone.txt")).unwrap();
    panel.load_entries().unwrap();
    // Should fall back to index 0
    assert_eq!(panel.selected(), Some(0));
}

// ==================== load_entries cleans invalid marks ====================

#[test]
fn test_load_entries_cleans_stale_marks() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("exists.txt"), b"").unwrap();

    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    panel.marked.insert(FileName::from("exists.txt"));
    panel.marked.insert(FileName::from("deleted.txt")); // stale mark

    panel.load_entries().unwrap();
    assert!(panel.marked.contains("exists.txt"));
    assert!(!panel.marked.contains("deleted.txt"));
}
