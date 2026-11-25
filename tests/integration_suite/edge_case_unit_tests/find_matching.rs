//! Purpose: Quick-find matching: case, empty prefix, `..` and wrap-around.

use super::*;

// ---------------------------------------------------------------------------
// 5. Panel::find_first_matching — case insensitivity and edge inputs
// ---------------------------------------------------------------------------

#[test]
fn test_find_first_matching_case_insensitive() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    File::create(temp_dir.path().join("Alpha.txt"))?;
    File::create(temp_dir.path().join("beta.txt"))?;

    let mut panel = Panel::new(temp_dir.path().to_path_buf())?;
    // Search with different case
    assert!(panel.find_first_matching("ALPHA"));
    let selected = panel.get_selected_entry().unwrap();
    assert_eq!(selected.name, "Alpha.txt");

    Ok(())
}

#[test]
fn test_find_first_matching_returns_false_for_empty_prefix() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    File::create(temp_dir.path().join("a.txt"))?;

    let mut panel = Panel::new(temp_dir.path().to_path_buf())?;
    assert!(!panel.find_first_matching(""));

    Ok(())
}

#[test]
fn test_find_first_matching_skips_dotdot() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let sub = temp_dir.path().join("sub");
    fs::create_dir(&sub)?;
    // No files named starting with "." except ".."
    let mut panel = Panel::new(sub)?;
    // ".." should exist; search for ".." should not match it
    assert!(!panel.find_first_matching(".."));

    Ok(())
}

// ---------------------------------------------------------------------------
// 6. Panel::find_next_matching — wrapping from end to beginning
// ---------------------------------------------------------------------------

#[test]
fn test_find_next_matching_wraps_around() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    File::create(temp_dir.path().join("apple.txt"))?;
    File::create(temp_dir.path().join("banana.txt"))?;
    File::create(temp_dir.path().join("avocado.txt"))?;

    let mut panel = Panel::new(temp_dir.path().to_path_buf())?;
    // Select first "a" match
    assert!(panel.find_first_matching("a"));
    let first = panel.selected().unwrap();

    // Find next "a" match — should wrap around if needed
    assert!(panel.find_next_matching("a"));
    let second = panel.selected().unwrap();
    assert_ne!(first, second);

    Ok(())
}
