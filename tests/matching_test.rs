use funny_file_manager::models::{DirEntry, Panel, SortMode};
use ratatui::widgets::ListState;
use std::collections::BTreeSet;
use std::path::PathBuf;

#[test]
fn test_panel_find_next_matching_multiple() {
    let mut panel = Panel {
        path: PathBuf::from("."),
        entries: vec![
            DirEntry {
                name: "..".to_string(),
                is_dir: true,
                size: 0,
            },
            DirEntry {
                name: "apple.txt".to_string(),
                is_dir: false,
                size: 10,
            },
            DirEntry {
                name: "apricot.txt".to_string(),
                is_dir: false,
                size: 20,
            },
            DirEntry {
                name: "banana.txt".to_string(),
                is_dir: false,
                size: 30,
            },
            DirEntry {
                name: "avocado.txt".to_string(),
                is_dir: false,
                size: 40,
            },
        ],
        state: ListState::default(),
        marked: BTreeSet::new(),
        sort_mode: SortMode::Name,
    };

    // First match for "a"
    panel.state.select(Some(0));
    assert!(panel.find_next_matching("a"));
    assert_eq!(panel.state.selected(), Some(1)); // apple.txt
    assert_eq!(
        panel.entries[panel.state.selected().unwrap()].name,
        "apple.txt"
    );

    // Second match for "a"
    assert!(panel.find_next_matching("a"));
    assert_eq!(panel.state.selected(), Some(2)); // apricot.txt

    // Third match for "a"
    assert!(panel.find_next_matching("a"));
    assert_eq!(panel.state.selected(), Some(4)); // avocado.txt

    // Wrap around to first match
    assert!(panel.find_next_matching("a"));
    assert_eq!(panel.state.selected(), Some(1)); // apple.txt
}

#[test]
fn test_panel_find_first_matching_case_insensitive() {
    let mut panel = Panel {
        path: PathBuf::from("."),
        entries: vec![
            DirEntry {
                name: "..".to_string(),
                is_dir: true,
                size: 0,
            },
            DirEntry {
                name: "Apple.txt".to_string(),
                is_dir: false,
                size: 10,
            },
            DirEntry {
                name: "banana.txt".to_string(),
                is_dir: false,
                size: 30,
            },
        ],
        state: ListState::default(),
        marked: BTreeSet::new(),
        sort_mode: SortMode::Name,
    };

    assert!(panel.find_first_matching("a"));
    assert_eq!(panel.state.selected(), Some(1));

    assert!(panel.find_first_matching("B"));
    assert_eq!(panel.state.selected(), Some(2));
}

#[test]
fn test_panel_find_matching_no_match() {
    let mut panel = Panel {
        path: PathBuf::from("."),
        entries: vec![
            DirEntry {
                name: "..".to_string(),
                is_dir: true,
                size: 0,
            },
            DirEntry {
                name: "apple.txt".to_string(),
                is_dir: false,
                size: 10,
            },
        ],
        state: ListState::default(),
        marked: BTreeSet::new(),
        sort_mode: SortMode::Name,
    };

    assert!(!panel.find_first_matching("xyz"));
    assert!(!panel.find_next_matching("xyz"));
}
