//! Purpose: Covers panel matching and selection behavior.
//!
use funny_file_manager::models::EntryKind;
use funny_file_manager::models::FileSize;
use funny_file_manager::models::Modified;
use funny_file_manager::models::{DirEntry, Panel};
use std::path::PathBuf;

#[test]
fn test_panel_find_next_matching_multiple() {
    let mut panel = Panel::from_entries(
        PathBuf::from("."),
        vec![
            entry("..", EntryKind::Directory, 0),
            entry("apple.txt", EntryKind::File, 10),
            entry("apricot.txt", EntryKind::File, 20),
            entry("banana.txt", EntryKind::File, 30),
            entry("avocado.txt", EntryKind::File, 40),
        ],
    );

    // First match for "a"
    panel.select(Some(0));
    assert!(panel.find_next_matching("a"));
    assert_eq!(panel.selected(), Some(1)); // apple.txt
    assert_eq!(panel.entries[panel.selected().unwrap()].name, "apple.txt");

    // Second match for "a"
    assert!(panel.find_next_matching("a"));
    assert_eq!(panel.selected(), Some(2)); // apricot.txt

    // Third match for "a"
    assert!(panel.find_next_matching("a"));
    assert_eq!(panel.selected(), Some(4)); // avocado.txt

    // Wrap around to first match
    assert!(panel.find_next_matching("a"));
    assert_eq!(panel.selected(), Some(1)); // apple.txt
}

#[test]
fn test_panel_find_first_matching_case_insensitive() {
    let mut panel = Panel::from_entries(
        PathBuf::from("."),
        vec![
            entry("..", EntryKind::Directory, 0),
            entry("Apple.txt", EntryKind::File, 10),
            entry("banana.txt", EntryKind::File, 30),
        ],
    );

    assert!(panel.find_first_matching("a"));
    assert_eq!(panel.selected(), Some(1));

    assert!(panel.find_first_matching("B"));
    assert_eq!(panel.selected(), Some(2));
}

#[test]
fn test_panel_find_matching_no_match() {
    let mut panel = Panel::from_entries(
        PathBuf::from("."),
        vec![
            entry("..", EntryKind::Directory, 0),
            entry("apple.txt", EntryKind::File, 10),
        ],
    );

    assert!(!panel.find_first_matching("xyz"));
    assert!(!panel.find_next_matching("xyz"));
}

fn entry(name: &str, kind: EntryKind, bytes: u64) -> DirEntry {
    DirEntry {
        name: name.to_owned(),
        kind,
        size: FileSize::from_bytes(bytes),
        modified: Modified::UNKNOWN,
        executable: false,
        ..DirEntry::default()
    }
}
