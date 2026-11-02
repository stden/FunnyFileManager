//! Purpose: Sort modes: cycling, labels, defaults and ordering by extension or size.

use super::*;

// ==================== Sort Modes ====================

#[test]
fn test_sort_mode_cycle() {
    assert_eq!(SortMode::Name.next(), SortMode::Extension);
    assert_eq!(SortMode::Extension.next(), SortMode::Size);
    assert_eq!(SortMode::Size.next(), SortMode::Name);
}

#[test]
fn test_sort_mode_labels() {
    assert_eq!(SortMode::Name.label(), "Name");
    assert_eq!(SortMode::Extension.label(), "Ext");
    assert_eq!(SortMode::Size.label(), "Size");
}

#[test]
fn test_sort_by_extension() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("file.zzz"), b"").unwrap();
    fs::write(temp.path().join("file.aaa"), b"").unwrap();
    fs::write(temp.path().join("noext"), b"").unwrap();

    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    panel.sort_mode = SortMode::Extension;
    panel.load_entries().unwrap();

    let file_names: Vec<&str> = panel
        .entries
        .iter()
        .filter(|e| !e.kind.is_dir())
        .map(|e| e.name.as_str())
        .collect();
    // No extension should come first, then aaa, then zzz
    assert_eq!(file_names, vec!["noext", "file.aaa", "file.zzz"]);
}

#[test]
fn test_sort_by_size() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("small.txt"), b"a").unwrap();
    fs::write(temp.path().join("big.txt"), b"aaaaaaaaaa").unwrap();
    fs::write(temp.path().join("medium.txt"), b"aaaaa").unwrap();

    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    panel.sort_mode = SortMode::Size;
    panel.load_entries().unwrap();

    let file_names: Vec<&str> = panel
        .entries
        .iter()
        .filter(|e| !e.kind.is_dir())
        .map(|e| e.name.as_str())
        .collect();
    // Largest first
    assert_eq!(file_names, vec!["big.txt", "medium.txt", "small.txt"]);
}

#[test]
fn test_cycle_sort_mode_method() {
    let temp = TempDir::new().unwrap();
    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    assert_eq!(panel.sort_mode, SortMode::Name);
    panel.cycle_sort_mode();
    assert_eq!(panel.sort_mode, SortMode::Extension);
    panel.cycle_sort_mode();
    assert_eq!(panel.sort_mode, SortMode::Size);
    panel.cycle_sort_mode();
    assert_eq!(panel.sort_mode, SortMode::Name);
}

// ==================== SortMode Default ====================

#[test]
fn test_sort_mode_default() {
    let mode: SortMode = SortMode::default();
    assert_eq!(mode, SortMode::Name);
}
