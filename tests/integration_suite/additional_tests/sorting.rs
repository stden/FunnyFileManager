//! Purpose: Sort modes: cycling, labels, default and ordering by extension or size.

use super::*;

// ---------------------------------------------------------------------------
// SortMode cycle and labels
// ---------------------------------------------------------------------------

#[test]
fn test_sort_mode_full_cycle() {
    let start = SortMode::Name;
    let ext = start.next();
    assert_eq!(ext, SortMode::Extension);
    let size = ext.next();
    assert_eq!(size, SortMode::Size);
    let name = size.next();
    assert_eq!(name, SortMode::Name);
}

#[test]
fn test_sort_mode_labels() {
    assert_eq!(SortMode::Name.label(), "Name");
    assert_eq!(SortMode::Extension.label(), "Ext");
    assert_eq!(SortMode::Size.label(), "Size");
}

#[test]
fn test_sort_mode_default() {
    let mode: SortMode = SortMode::default();
    assert_eq!(mode, SortMode::Name);
}

// ---------------------------------------------------------------------------
// Panel sorting
// ---------------------------------------------------------------------------

#[test]
fn test_panel_sort_by_extension() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("c.rs"), "c").unwrap();
    fs::write(temp.path().join("a.txt"), "a").unwrap();
    fs::write(temp.path().join("b.md"), "b").unwrap();

    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    panel.sort_mode = SortMode::Extension;
    panel.load_entries().unwrap();

    let file_names: Vec<&str> = panel
        .entries
        .iter()
        .filter(|e| !e.kind.is_dir())
        .map(|e| e.name.as_str())
        .collect();
    // Extension sort: .md < .rs < .txt
    assert_eq!(file_names, vec!["b.md", "c.rs", "a.txt"]);
}

#[test]
fn test_panel_sort_by_size() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("small.txt"), "x").unwrap();
    fs::write(temp.path().join("big.txt"), "x".repeat(1000)).unwrap();
    fs::write(temp.path().join("medium.txt"), "x".repeat(100)).unwrap();

    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    panel.sort_mode = SortMode::Size;
    panel.load_entries().unwrap();

    let file_names: Vec<&str> = panel
        .entries
        .iter()
        .filter(|e| !e.kind.is_dir())
        .map(|e| e.name.as_str())
        .collect();
    // Size sort is descending: big > medium > small
    assert_eq!(file_names, vec!["big.txt", "medium.txt", "small.txt"]);
}

// ---------------------------------------------------------------------------
// Panel cycle_sort_mode
// ---------------------------------------------------------------------------

#[test]
fn test_panel_cycle_sort_mode() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("a.txt"), "x").unwrap();

    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    assert_eq!(panel.sort_mode, SortMode::Name);

    panel.cycle_sort_mode();
    assert_eq!(panel.sort_mode, SortMode::Extension);

    panel.cycle_sort_mode();
    assert_eq!(panel.sort_mode, SortMode::Size);

    panel.cycle_sort_mode();
    assert_eq!(panel.sort_mode, SortMode::Name);
}
