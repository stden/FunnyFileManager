//! Purpose: unit tests for Enter on a panel row, and selecting by name.

use super::*;

#[test]
fn test_panel_enter_directory() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let subdir = temp_dir.path().join("subdir");
    fs::create_dir(&subdir).expect("test setup");

    let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    // Find and select the subdirectory
    for (i, entry) in panel.entries.iter().enumerate() {
        if entry.name == "subdir" {
            panel.select(Some(i));
            break;
        }
    }

    // Enter the directory
    panel.enter().expect("test setup");
    assert_eq!(panel.path, subdir);

    // Should have .. entry
    assert!(panel.entries.iter().any(|e| e.name == ".."));

    // Go back to parent
    panel.select(Some(0)); // Select ..
    panel.enter().expect("test setup");
    assert_eq!(panel.path, temp_dir.path());

    Ok(())
}

#[test]
fn test_panel_enter_clears_marks() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let subdir = temp_dir.path().join("subdir");
    fs::create_dir(&subdir).expect("test setup");
    File::create(temp_dir.path().join("file.txt")).expect("test setup");

    let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    let file_index = panel
        .entries
        .iter()
        .position(|entry| entry.name == "file.txt")
        .expect("file.txt entry");
    panel.select(Some(file_index));
    assert!(panel.toggle_mark_selected());
    assert!(panel.marked.contains("file.txt"));

    let dir_index = panel
        .entries
        .iter()
        .position(|entry| entry.name == "subdir")
        .expect("subdir entry");
    panel.select(Some(dir_index));
    panel.enter().expect("test setup");

    assert_eq!(panel.marked.len(), 0);

    Ok(())
}

#[test]
fn test_panel_enter_file() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    File::create(temp_dir.path().join("test.txt")).expect("test setup");

    let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    let original_path = panel.path.clone();

    // Find and select the file
    for (i, entry) in panel.entries.iter().enumerate() {
        if entry.name == "test.txt" {
            panel.select(Some(i));
            break;
        }
    }

    // Entering a file should not change the path
    panel.enter().expect("test setup");
    assert_eq!(panel.path, original_path);

    Ok(())
}

#[test]
fn test_panel_enter_no_selection() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    panel.select(None);

    panel.enter().expect("test setup");
    assert_eq!(panel.path, temp_dir.path());

    Ok(())
}

#[test]
fn test_panel_enter_missing_entry() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    panel.select(Some(10_000));

    panel.enter().expect("test setup");
    assert_eq!(panel.path, temp_dir.path());

    Ok(())
}

// ─── selecting by name, and entering nothing ────────────────────────────────

/// The caller has to be able to tell "there it is" from "the listing does not
/// show it" — which happens when the thing just created went somewhere the
/// panel is not looking.
#[test]
fn selecting_a_name_the_listing_does_not_have_reports_it_and_moves_nothing() {
    let dir = TempDir::new().expect("test setup");
    fs::write(dir.path().join("here.txt"), b"x").expect("test setup");
    let mut panel = Panel::new(dir.path().to_path_buf()).expect("panel");

    assert!(panel.select_named("here.txt"));
    let landed = panel.selected();

    assert!(!panel.select_named("elsewhere.txt"), "not in this listing");
    assert_eq!(panel.selected(), landed, "and the cursor did not move");
}

/// Enter with the cursor on nothing does nothing, rather than reaching for a
/// row that is not there.
#[test]
fn entering_with_no_row_under_the_cursor_does_nothing() {
    let dir = TempDir::new().expect("test setup");
    let mut panel = Panel::new(dir.path().to_path_buf()).expect("panel");
    let path = panel.path.clone();

    panel.select(None);
    panel.enter().expect("nothing to do is not a failure");
    assert_eq!(panel.path, path);
}
