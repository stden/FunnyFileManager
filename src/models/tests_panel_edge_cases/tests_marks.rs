//! Purpose: unit tests for marks and the names a file operation acts on.

use super::*;

#[test]
fn test_action_entry_names_parent_directory() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    File::create(temp_dir.path().join("file.txt")).expect("test setup");
    panel.load_entries().expect("test setup");

    // Select parent directory ".."
    panel.select(Some(0));
    let names = panel.action_entry_names();

    // Should be empty when parent directory is selected and no marks
    assert!(names.is_empty());

    Ok(())
}

#[test]
fn test_panel_action_entry_names_with_special_cases() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    File::create(temp_dir.path().join("regular.txt")).expect("test setup");
    File::create(temp_dir.path().join(".hidden")).expect("test setup");
    fs::create_dir(temp_dir.path().join("directory")).expect("test setup");

    panel.load_entries().expect("test setup");

    let names = panel.action_entry_names();
    assert!(names.is_empty());

    panel.select(Some(1));
    let names = panel.action_entry_names();
    assert_eq!(names.len(), 1);

    panel.marked.insert(FileName::from("regular.txt"));
    panel.marked.insert(FileName::from(".hidden"));
    panel.marked.insert(FileName::from("directory"));

    let names = panel.action_entry_names();
    assert_eq!(names.len(), 3);
    assert!(names.contains(&"regular.txt".to_string()));
    assert!(names.contains(&".hidden".to_string()));
    assert!(names.contains(&"directory".to_string()));

    Ok(())
}

/// `action_entry_names` returns empty when no marks are set and the
/// selected entry is ".." (parent directory).
#[test]
fn test_action_entry_names_no_marks_dotdot_selected_returns_empty() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    File::create(temp_dir.path().join("file.txt")).expect("test setup");

    let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    // ".." is always the first entry when a parent exists.
    let dotdot_index = panel
        .entries
        .iter()
        .position(|e| e.name == "..")
        .expect(".. must exist");
    panel.select(Some(dotdot_index));

    let names = panel.action_entry_names();
    assert!(
        names.is_empty(),
        "action_entry_names should return empty when '..' is selected and no marks set"
    );

    Ok(())
}

/// `action_entry_names` returns all marked file names when marks are present,
/// regardless of what is currently selected.
#[test]
fn test_action_entry_names_with_multiple_marks() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    File::create(temp_dir.path().join("a.txt")).expect("test setup");
    File::create(temp_dir.path().join("b.txt")).expect("test setup");
    File::create(temp_dir.path().join("c.txt")).expect("test setup");

    let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    panel.marked.insert(FileName::from("a.txt"));
    panel.marked.insert(FileName::from("c.txt"));

    // Select "b.txt" so that we can confirm the selection is ignored when marks exist.
    let b_idx = panel
        .entries
        .iter()
        .position(|e| e.name == "b.txt")
        .expect("b.txt must exist");
    panel.select(Some(b_idx));

    let mut names = panel.action_entry_names();
    names.sort();
    assert_eq!(names, vec!["a.txt", "c.txt"]);

    Ok(())
}

/// `toggle_mark_selected` marks then unmarks the same entry.
#[test]
fn test_toggle_mark_selected_marks_then_unmarks() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    File::create(temp_dir.path().join("target.txt")).expect("test setup");

    let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    let idx = panel
        .entries
        .iter()
        .position(|e| e.name == "target.txt")
        .expect("target.txt must exist");
    panel.select(Some(idx));

    // First toggle: marks the file.
    assert!(panel.toggle_mark_selected());
    assert!(panel.marked.contains("target.txt"), "file should be marked");

    // Second toggle: unmarks the file.
    assert!(panel.toggle_mark_selected());
    assert!(
        !panel.marked.contains("target.txt"),
        "file should be unmarked"
    );

    Ok(())
}
