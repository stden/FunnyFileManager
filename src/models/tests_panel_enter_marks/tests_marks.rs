//! Purpose: unit tests for marks surviving a reload and winning over the cursor.

use super::*;

/// `action_entry_names` with mixed marks and selection
#[test]
fn test_action_entry_names_returns_marked_over_selected() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    fs::write(temp_dir.path().join("a.txt"), "a")?;
    fs::write(temp_dir.path().join("b.txt"), "b")?;

    let mut panel = Panel::new(temp_dir.path().to_path_buf())?;
    panel.marked.insert(FileName::from("a.txt"));
    // Select b.txt
    for (i, entry) in panel.entries.iter().enumerate() {
        if entry.name == "b.txt" {
            panel.select(Some(i));
            break;
        }
    }

    let names = panel.action_entry_names();
    assert_eq!(names, vec!["a.txt"], "marked should take precedence");

    Ok(())
}

// Kill: replace != with == in marked retention filter
#[test]
fn test_load_entries_retains_marks_for_existing_files() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    fs::write(temp_dir.path().join("keep.txt"), "data")?;
    fs::write(temp_dir.path().join("also.txt"), "data")?;

    let mut panel = Panel::new(temp_dir.path().to_path_buf())?;
    panel.marked.insert(FileName::from("keep.txt"));
    panel.marked.insert(FileName::from("also.txt"));
    panel.marked.insert(FileName::from("gone.txt")); // file doesn't exist

    panel.load_entries()?;

    // Existing files are retained, missing file is removed
    assert!(
        panel.marked.contains("keep.txt"),
        "existing marked file should be retained"
    );
    assert!(
        panel.marked.contains("also.txt"),
        "existing marked file should be retained"
    );
    assert!(
        !panel.marked.contains("gone.txt"),
        "missing marked file should be removed"
    );
    // ".." should never be in marked
    assert!(
        !panel.marked.contains(".."),
        "parent entry should not be in marks"
    );

    Ok(())
}
