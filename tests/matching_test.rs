//! Purpose: Covers panel matching and selection behavior.
//!
use funny_file_manager::models::EntryKind;
use funny_file_manager::models::FileSize;
use funny_file_manager::models::{DirEntry, Panel};
use std::path::PathBuf;

#[test]
fn test_panel_find_next_matching_multiple() {
    let mut panel = Panel::from_entries(
        PathBuf::from("."),
        vec![
            DirEntry {
                name: "..".to_string(),
                kind: EntryKind::Directory,
                size: FileSize::from_bytes(0),
            },
            DirEntry {
                name: "apple.txt".to_string(),
                kind: EntryKind::File,
                size: FileSize::from_bytes(10),
            },
            DirEntry {
                name: "apricot.txt".to_string(),
                kind: EntryKind::File,
                size: FileSize::from_bytes(20),
            },
            DirEntry {
                name: "banana.txt".to_string(),
                kind: EntryKind::File,
                size: FileSize::from_bytes(30),
            },
            DirEntry {
                name: "avocado.txt".to_string(),
                kind: EntryKind::File,
                size: FileSize::from_bytes(40),
            },
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
            DirEntry {
                name: "..".to_string(),
                kind: EntryKind::Directory,
                size: FileSize::from_bytes(0),
            },
            DirEntry {
                name: "Apple.txt".to_string(),
                kind: EntryKind::File,
                size: FileSize::from_bytes(10),
            },
            DirEntry {
                name: "banana.txt".to_string(),
                kind: EntryKind::File,
                size: FileSize::from_bytes(30),
            },
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
            DirEntry {
                name: "..".to_string(),
                kind: EntryKind::Directory,
                size: FileSize::from_bytes(0),
            },
            DirEntry {
                name: "apple.txt".to_string(),
                kind: EntryKind::File,
                size: FileSize::from_bytes(10),
            },
        ],
    );

    assert!(!panel.find_first_matching("xyz"));
    assert!(!panel.find_next_matching("xyz"));
}
