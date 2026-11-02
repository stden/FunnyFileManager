//! Purpose: Quick-search matching: first match, next match, wrap-around and case.

use super::*;

// ==================== Find Matching ====================

#[test]
fn test_find_first_matching_empty_prefix() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("a.txt"), b"").unwrap();
    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    assert!(!panel.find_first_matching(""));
}

#[test]
fn test_find_first_matching_empty_panel() {
    let mut panel = Panel::from_entries(PathBuf::from("/tmp"), vec![]);
    assert!(!panel.find_first_matching("x"));
}

#[test]
fn test_find_first_matching_case_insensitive() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("Alpha.txt"), b"").unwrap();
    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    assert!(panel.find_first_matching("alpha"));
    let selected = panel.get_selected_entry().unwrap();
    assert_eq!(selected.name, "Alpha.txt");
}

#[test]
fn test_find_next_matching_wraps() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("aa.txt"), b"").unwrap();
    fs::write(temp.path().join("ab.txt"), b"").unwrap();

    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    // Find first "a" match
    assert!(panel.find_first_matching("a"));
    let first = panel.selected().unwrap();
    // Find next "a" match
    assert!(panel.find_next_matching("a"));
    let second = panel.selected().unwrap();
    assert_ne!(first, second);
    // Find next again should wrap back
    assert!(panel.find_next_matching("a"));
}

#[test]
fn test_find_next_matching_no_match() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("a.txt"), b"").unwrap();
    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    assert!(!panel.find_next_matching("zzz"));
}

#[test]
fn test_find_next_matching_empty_prefix() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("a.txt"), b"").unwrap();
    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    assert!(!panel.find_next_matching(""));
}
