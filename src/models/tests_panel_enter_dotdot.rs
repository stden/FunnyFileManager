//! Unit tests: entering `..` from a panel path whose last component is itself
//! `..`, so there is no directory name to return the cursor to.

use std::fs;

use tempfile::TempDir;

use super::*;

/// `Path::file_name` reports `None` for a path ending in `..`, so `enter()` has
/// no "directory we came from" to re-select. It must still pop the path, reload
/// the listing for the new location and leave a usable selection.
#[test]
fn test_enter_parent_without_a_file_name_navigates_and_selects_first() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let nested = temp_dir.path().join("sub").join("nested");
    fs::create_dir_all(&nested).expect("test setup");
    fs::write(nested.join("inside.txt"), b"in").expect("test setup");
    fs::write(temp_dir.path().join("sub").join("outside.txt"), b"out").expect("test setup");

    // `sub/nested/..` resolves to `sub` on disk, so the panel lists `sub`, but
    // textually its last component is `..` — the case with no `file_name`.
    let mut panel = Panel::new(nested.join(".."))?;
    let dotdot = panel
        .entries
        .iter()
        .position(|entry| entry.name == "..")
        .expect("test setup: `..` entry present");
    panel.select(Some(dotdot));
    assert!(
        panel
            .entries
            .iter()
            .any(|entry| entry.name == "outside.txt"),
        "test setup: the panel starts on the contents of `sub`"
    );

    panel.enter()?;

    assert_eq!(
        panel.path, nested,
        "the trailing `..` is popped off the path"
    );
    let names: Vec<&str> = panel
        .entries
        .iter()
        .map(|entry| entry.name.as_str())
        .collect();
    assert_eq!(
        names,
        ["..", "inside.txt"],
        "the listing follows the new path"
    );
    assert_eq!(panel.selected(), Some(0));
    Ok(())
}
