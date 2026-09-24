//! Purpose: unit tests for creating and loading a panel, and the selection a
//! reload keeps.

use super::*;

#[test]
fn test_panel_new() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    assert_eq!(panel.path, temp_dir.path());
    assert!(!panel.entries.is_empty());
    assert_eq!(panel.selected(), Some(0));

    panel.entries.clear();
    panel.select(None);
    assert!(panel.entries.is_empty());
    assert_eq!(panel.selected(), None);

    Ok(())
}

#[test]
fn test_panel_load_entries_empty_dir() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    panel.load_entries().expect("test setup");

    assert!(panel.entries.iter().any(|e| e.name == ".."));

    Ok(())
}

#[test]
fn test_panel_load_entries_preserves_selection() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    File::create(temp_dir.path().join("a.txt"))
        .expect("test setup")
        .write_all(b"a")
        .expect("test setup");
    File::create(temp_dir.path().join("b.txt"))
        .expect("test setup")
        .write_all(b"b")
        .expect("test setup");

    let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    let selected_index = panel
        .entries
        .iter()
        .position(|entry| entry.name == "b.txt")
        .expect("b.txt entry");
    panel.select(Some(selected_index));

    File::create(temp_dir.path().join("aa.txt"))
        .expect("test setup")
        .write_all(b"aa")
        .expect("test setup");
    panel.load_entries().expect("test setup");

    let selected = panel.get_selected_entry().expect("selected entry");
    assert_eq!(selected.name, "b.txt");

    Ok(())
}

#[test]
fn test_panel_load_entries_stays_put_when_selected_missing() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    File::create(temp_dir.path().join("a.txt"))
        .expect("test setup")
        .write_all(b"a")
        .expect("test setup");
    File::create(temp_dir.path().join("b.txt"))
        .expect("test setup")
        .write_all(b"b")
        .expect("test setup");

    let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    let selected_index = panel
        .entries
        .iter()
        .position(|entry| entry.name == "b.txt")
        .expect("b.txt entry");
    panel.select(Some(selected_index));

    fs::remove_file(temp_dir.path().join("b.txt")).expect("test setup");
    panel.load_entries().expect("test setup");

    // Far leaves the cursor on the row the deleted entry had, so whatever
    // moved up into it is under the cursor — not the top of the listing.
    assert_eq!(
        panel.selected(),
        Some(selected_index.min(panel.entries.len() - 1))
    );

    Ok(())
}
