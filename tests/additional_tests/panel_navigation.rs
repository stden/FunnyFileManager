//! Purpose: Cursor movement, mark toggles, Enter on a file and the selected path.

use super::*;

// ---------------------------------------------------------------------------
// Panel navigation edge cases
// ---------------------------------------------------------------------------

#[test]
fn test_panel_next_stops_at_the_last_entry() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("a.txt"), "a").unwrap();

    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    let last = panel.entries.len() - 1;
    panel.select(Some(last));

    panel.next();
    assert_eq!(
        panel.selected(),
        Some(panel.entries.len() - 1),
        "down at the bottom stays at the bottom"
    );
}

#[test]
fn test_panel_previous_stops_at_the_first_entry() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("a.txt"), "a").unwrap();

    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    panel.select(Some(0));

    panel.previous();
    assert_eq!(panel.selected(), Some(0), "up at the top stays at the top");
}

#[test]
fn test_panel_next_empty_panel() {
    let temp = TempDir::new().unwrap();
    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    panel.entries.clear();
    panel.select(None);

    panel.next();
    // Should not panic, selection stays None
    assert_eq!(panel.selected(), None);
}

// ---------------------------------------------------------------------------
// Panel toggle_mark edge cases
// ---------------------------------------------------------------------------

#[test]
fn test_toggle_mark_dotdot_is_noop() {
    let temp = TempDir::new().unwrap();
    fs::create_dir(temp.path().join("sub")).unwrap();

    let mut panel = Panel::new(temp.path().join("sub")).unwrap();
    // Select ".." entry
    panel.select(Some(0));
    assert_eq!(panel.entries[0].name, "..");

    let result = panel.toggle_mark_selected();
    assert!(!result, "marking '..' should return false");
    assert!(panel.marked.is_empty());
}

#[test]
fn test_toggle_mark_and_unmark() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("file.txt"), "x").unwrap();

    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    let pos = panel
        .entries
        .iter()
        .position(|e| e.name == "file.txt")
        .unwrap();
    panel.select(Some(pos));

    // Mark
    assert!(panel.toggle_mark_selected());
    assert!(panel.marked.contains("file.txt"));

    // Unmark
    assert!(panel.toggle_mark_selected());
    assert!(!panel.marked.contains("file.txt"));
}

// ---------------------------------------------------------------------------
// Panel enter() on file (not dir) is noop
// ---------------------------------------------------------------------------

#[test]
fn test_panel_enter_file_is_noop() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("data.txt"), "x").unwrap();

    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    let pos = panel
        .entries
        .iter()
        .position(|e| e.name == "data.txt")
        .unwrap();
    panel.select(Some(pos));

    let path_before = panel.path.clone();
    panel.enter().unwrap();
    assert_eq!(
        panel.path, path_before,
        "entering a file should not change path"
    );
}

// ---------------------------------------------------------------------------
// Panel selected_path returns None for ".."
// ---------------------------------------------------------------------------

#[test]
fn test_panel_selected_path_dotdot_returns_none() {
    let temp = TempDir::new().unwrap();
    let sub = temp.path().join("sub");
    fs::create_dir(&sub).unwrap();

    let mut panel = Panel::new(sub).unwrap();
    panel.select(Some(0)); // ".."
    assert!(panel.selected_path().is_none());
}

#[test]
fn test_panel_selected_path_normal_file() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("file.txt"), "x").unwrap();

    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    let pos = panel
        .entries
        .iter()
        .position(|e| e.name == "file.txt")
        .unwrap();
    panel.select(Some(pos));

    let path = panel.selected_path().unwrap();
    assert_eq!(path, temp.path().join("file.txt"));
}
