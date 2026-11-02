//! Purpose: Quick-find matching and the entries an action applies to.

use super::*;

// ---------------------------------------------------------------------------
// Panel find_first_matching / find_next_matching
// ---------------------------------------------------------------------------

#[test]
fn test_find_first_matching_case_insensitive() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("README.md"), "x").unwrap();

    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    panel.find_first_matching("read");

    let selected = panel.get_selected_entry().unwrap();
    assert_eq!(selected.name, "README.md");
}

#[test]
fn test_find_first_matching_no_match() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("a.txt"), "x").unwrap();

    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    let before = panel.selected();
    panel.find_first_matching("zzzzz");

    // Selection should not change
    assert_eq!(panel.selected(), before);
}

#[test]
fn test_find_next_matching_wraps() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("aa.txt"), "x").unwrap();
    fs::write(temp.path().join("ab.txt"), "x").unwrap();

    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    panel.find_first_matching("a");
    let first = panel.selected();

    panel.find_next_matching("a");
    let second = panel.selected();
    assert_ne!(first, second, "should advance to next match");

    // Keep calling find_next until it wraps
    for _ in 0..panel.entries.len() {
        panel.find_next_matching("a");
    }
}

// ---------------------------------------------------------------------------
// Panel action_entry_names
// ---------------------------------------------------------------------------

#[test]
fn test_action_entry_names_with_marks() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("a.txt"), "a").unwrap();
    fs::write(temp.path().join("b.txt"), "b").unwrap();

    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    panel.marked.insert(FileName::from("a.txt"));
    panel.marked.insert(FileName::from("b.txt"));

    let names = panel.action_entry_names();
    assert!(names.contains(&"a.txt".to_owned()));
    assert!(names.contains(&"b.txt".to_owned()));
}

#[test]
fn test_action_entry_names_no_marks_uses_selection() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("only.txt"), "x").unwrap();

    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    let pos = panel
        .entries
        .iter()
        .position(|e| e.name == "only.txt")
        .unwrap();
    panel.select(Some(pos));

    let names = panel.action_entry_names();
    assert_eq!(names, vec!["only.txt"]);
}
