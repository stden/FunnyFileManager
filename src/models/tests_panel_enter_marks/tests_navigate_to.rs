//! Purpose: unit tests for `Panel::navigate_to` — marks move with the path, failures roll back.

use super::*;

// -----------------------------------------------------------------------
// navigate_to: path and marks move together, failures roll back
// -----------------------------------------------------------------------

/// Marks are bare names, so they must never outlive the path they were taken in.
#[test]
fn test_navigate_to_clears_marks() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let other = temp_dir.path().join("other");
    fs::create_dir(&other)?;
    fs::write(temp_dir.path().join("tmp"), "here")?;
    fs::write(other.join("tmp"), "a different file with the same name")?;

    let mut panel = Panel::new(temp_dir.path().to_path_buf())?;
    panel.marked.insert(FileName::from("tmp"));

    panel.navigate_to(other.clone())?;

    assert_eq!(panel.path, other);
    assert_eq!(
        panel.marked.len(),
        0,
        "a name-keyed mark must not re-bind to a same-named file elsewhere"
    );
    assert!(panel.selected().is_some());

    Ok(())
}

/// A failed load must leave the panel on its previous, readable directory —
/// path, marks and selection included.
#[test]
fn test_navigate_to_failure_restores_previous_directory() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    fs::write(temp_dir.path().join("a.txt"), "a")?;

    let mut panel = Panel::new(temp_dir.path().to_path_buf())?;
    panel.marked.insert(FileName::from("a.txt"));
    let selection_before = panel.selected();

    let error = panel
        .navigate_to(temp_dir.path().join("does_not_exist"))
        .expect_err("missing directory must fail");

    assert_eq!(error.kind(), io::ErrorKind::NotFound);
    assert_eq!(panel.path, temp_dir.path());
    assert!(panel.entries.iter().any(|entry| entry.name == "a.txt"));
    assert_eq!(panel.selected(), selection_before);
    assert!(
        panel.marked.contains("a.txt"),
        "marks belong to the directory we never left"
    );

    Ok(())
}
