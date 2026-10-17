//! Purpose: unit tests for quick search (`find_first_matching` / `find_next_matching`).

use super::*;

#[test]
fn test_quick_search_find_first_matching() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    File::create(temp_dir.path().join("alpha.txt")).expect("test setup");
    File::create(temp_dir.path().join("beta.txt")).expect("test setup");
    File::create(temp_dir.path().join("gamma.txt")).expect("test setup");
    File::create(temp_dir.path().join("Alpha_upper.txt")).expect("test setup");

    let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    // Find first entry starting with "a" (case-insensitive)
    assert!(panel.find_first_matching("a"));
    let selected = panel.get_selected_entry().unwrap();
    assert!(selected.name.to_lowercase().starts_with('a'));

    // Find first entry starting with "b"
    assert!(panel.find_first_matching("b"));
    let selected = panel.get_selected_entry().unwrap();
    assert_eq!(selected.name, "beta.txt");

    // Non-existent prefix
    assert!(!panel.find_first_matching("z"));

    // Empty prefix
    assert!(!panel.find_first_matching(""));

    Ok(())
}

#[test]
fn test_quick_search_find_next_matching() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    File::create(temp_dir.path().join("apple.txt")).expect("test setup");
    File::create(temp_dir.path().join("apricot.txt")).expect("test setup");
    File::create(temp_dir.path().join("avocado.txt")).expect("test setup");
    File::create(temp_dir.path().join("banana.txt")).expect("test setup");

    let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    // Find first "a" entry
    assert!(panel.find_first_matching("a"));
    let first_match = panel.get_selected_entry().unwrap().name.clone();

    // Find next "a" entry - should cycle to a different one
    assert!(panel.find_next_matching("a"));
    let second_match = panel.get_selected_entry().unwrap().name.clone();
    assert_ne!(first_match, second_match);
    assert!(second_match.to_lowercase().starts_with('a'));

    Ok(())
}

#[test]
fn test_quick_search_empty_panel() {
    let mut panel = Panel::from_entries(PathBuf::new(), Vec::new());

    assert!(!panel.find_first_matching("a"));
    assert!(!panel.find_next_matching("a"));
}

#[test]
fn test_app_quick_search_field() -> io::Result<()> {
    let app = App::new().expect("test setup");
    assert!(app.quick_search.query().is_none());
    Ok(())
}

// -----------------------------------------------------------------------
// find_next_matching – additional branch coverage
// -----------------------------------------------------------------------

/// When there is exactly one matching entry, `find_next_matching` wraps
/// all the way around and returns true (selecting the same entry again).
#[test]
fn test_find_next_matching_single_match_wraps_to_itself() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    File::create(temp_dir.path().join("unique.txt")).expect("test setup");
    File::create(temp_dir.path().join("other.txt")).expect("test setup");

    let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    // Land on "unique.txt" first.
    assert!(panel.find_first_matching("unique"));
    let first_idx = panel.selected().expect("should be selected");

    // find_next_matching should wrap around and find the same entry.
    let found = panel.find_next_matching("unique");
    assert!(found, "should find the only match (wrapping)");
    assert_eq!(
        panel.selected(),
        Some(first_idx),
        "should wrap back to the same entry"
    );

    Ok(())
}

/// `find_next_matching` returns false when the prefix is empty.
#[test]
fn test_find_next_matching_empty_prefix_returns_false() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    File::create(temp_dir.path().join("file.txt")).expect("test setup");

    let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    let found = panel.find_next_matching("");
    assert!(!found, "empty prefix should return false");

    Ok(())
}

/// `find_next_matching` returns false when no entry matches.
#[test]
fn test_find_next_matching_no_match_returns_false() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    File::create(temp_dir.path().join("alpha.txt")).expect("test setup");

    let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    let found = panel.find_next_matching("zzz");
    assert!(!found, "non-matching prefix should return false");

    Ok(())
}
