//! Purpose: Marks and the selection across a reload, and loading a missing path.

use super::*;

// ---------------------------------------------------------------------------
// Marked files cleanup after reload
// ---------------------------------------------------------------------------

#[test]
fn test_marked_files_cleaned_on_reload() {
    let temp = TempDir::new().unwrap();
    let file_a = temp.path().join("a.txt");
    let file_b = temp.path().join("b.txt");
    fs::write(&file_a, "aaa").unwrap();
    fs::write(&file_b, "bbb").unwrap();

    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    panel.marked.insert(FileName::from("a.txt"));
    panel.marked.insert(FileName::from("b.txt"));
    assert_eq!(panel.marked.len(), 2);

    // Delete a.txt, reload — marked should drop "a.txt"
    fs::remove_file(&file_a).unwrap();
    panel.load_entries().unwrap();

    assert!(
        !panel.marked.contains("a.txt"),
        "deleted file should be unmarked"
    );
    assert!(
        panel.marked.contains("b.txt"),
        "existing file should stay marked"
    );
}

#[test]
fn test_marked_files_all_removed() {
    let temp = TempDir::new().unwrap();
    let file = temp.path().join("only.txt");
    fs::write(&file, "x").unwrap();

    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    panel.marked.insert(FileName::from("only.txt"));

    fs::remove_file(&file).unwrap();
    panel.load_entries().unwrap();

    assert_eq!(panel.marked.len(), 0, "all marks should be cleared");
}

// ---------------------------------------------------------------------------
// Panel selection after reload when file disappears
// ---------------------------------------------------------------------------

#[test]
fn test_selection_holds_its_row_when_selected_file_deleted() {
    let temp = TempDir::new().unwrap();
    let a = temp.path().join("a.txt");
    let b = temp.path().join("b.txt");
    fs::write(&a, "a").unwrap();
    fs::write(&b, "b").unwrap();

    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    // Select "b.txt"
    if let Some(pos) = panel.entries.iter().position(|e| e.name == "b.txt") {
        panel.select(Some(pos));
    }

    let row = panel.selected().expect("a selection to start from");
    fs::remove_file(&b).unwrap();
    panel.load_entries().unwrap();

    // Far keeps the cursor on the row the deleted entry had, so the next file
    // is under it. Only a listing that got shorter moves the cursor at all.
    let sel = panel.selected();
    assert!(sel.is_some(), "should have a selection");
    assert_eq!(
        sel.unwrap(),
        row.min(panel.entries.len() - 1),
        "the cursor holds its row rather than jumping to the top"
    );
}

#[test]
fn test_selection_preserved_when_selected_file_exists() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("x.txt"), "x").unwrap();
    fs::write(temp.path().join("y.txt"), "y").unwrap();

    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    // Find and select "y.txt"
    let pos = panel
        .entries
        .iter()
        .position(|e| e.name == "y.txt")
        .unwrap();
    panel.select(Some(pos));

    panel.load_entries().unwrap();

    let selected = panel.get_selected_entry().unwrap();
    assert_eq!(selected.name, "y.txt", "selection should be preserved");
}

// ---------------------------------------------------------------------------
// Panel load_entries with invalid path
// ---------------------------------------------------------------------------

#[test]
fn test_panel_load_entries_invalid_path() {
    let mut panel = Panel::from_entries(
        PathBuf::from("/nonexistent/path/that/does/not/exist"),
        Vec::new(),
    );
    let result = panel.load_entries();
    assert!(result.is_err(), "loading invalid path should error");
}

// ---------------------------------------------------------------------------
// Panel clear_marks
// ---------------------------------------------------------------------------

#[test]
fn test_panel_clear_marks() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("a.txt"), "a").unwrap();
    fs::write(temp.path().join("b.txt"), "b").unwrap();

    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    panel.marked.insert(FileName::from("a.txt"));
    panel.marked.insert(FileName::from("b.txt"));
    assert_eq!(panel.marked.len(), 2);

    panel.clear_marks();
    assert_eq!(panel.marked.len(), 0);
}
