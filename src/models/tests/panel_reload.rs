//! Purpose: Panel loading and where the cursor lands after a reload — it follows its name, or stays on its row.

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
fn test_select_after_reload_empty_entries_with_selected_name() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    panel.entries.clear();
    panel.select(Some(0));
    panel.select_after_reload(&Panel::cursor_for_test(Some("missing.txt"), 1));

    assert_eq!(panel.selected(), None);
    Ok(())
}

#[test]
fn test_select_after_reload_empty_entries_without_selected_name() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    panel.entries.clear();
    panel.select(Some(0));
    panel.select_after_reload(&Panel::cursor_for_test(None, 0));

    assert_eq!(panel.selected(), None);
    Ok(())
}

#[test]
fn test_select_after_reload_preserves_selected_name() {
    let mut panel = Panel::from_entries(
        PathBuf::new(),
        vec![
            DirEntry {
                name: "alpha.txt".to_owned(),
                kind: EntryKind::File,
                size: FileSize::from_bytes(10),
                modified: Modified::UNKNOWN,
                executable: false,
                ..DirEntry::default()
            },
            DirEntry {
                name: "beta.txt".to_owned(),
                kind: EntryKind::File,
                size: FileSize::from_bytes(20),
                modified: Modified::UNKNOWN,
                executable: false,
                ..DirEntry::default()
            },
        ],
    );

    panel.select(Some(0));
    panel.select_after_reload(&Panel::cursor_for_test(Some("beta.txt"), 0));

    assert_eq!(panel.selected(), Some(1));
}

#[test]
fn test_select_after_reload_stays_put_when_the_entry_is_gone() {
    let mut panel = Panel::from_entries(
        PathBuf::new(),
        vec![
            DirEntry {
                name: "alpha.txt".to_owned(),
                kind: EntryKind::File,
                size: FileSize::from_bytes(10),
                modified: Modified::UNKNOWN,
                executable: false,
                ..DirEntry::default()
            },
            DirEntry {
                name: "beta.txt".to_owned(),
                kind: EntryKind::File,
                size: FileSize::from_bytes(20),
                modified: Modified::UNKNOWN,
                executable: false,
                ..DirEntry::default()
            },
        ],
    );

    panel.select(Some(1));
    panel.select_after_reload(&Panel::cursor_for_test(Some("missing.txt"), 1));

    // Far leaves the cursor on the row the deleted entry occupied, so the
    // next file is under it — not back at the top of the listing.
    assert_eq!(panel.selected(), Some(1));
}

#[test]
fn test_select_after_reload_keeps_selection_when_none() {
    let mut panel = Panel::from_entries(
        PathBuf::new(),
        vec![
            DirEntry {
                name: "alpha.txt".to_owned(),
                kind: EntryKind::File,
                size: FileSize::from_bytes(10),
                modified: Modified::UNKNOWN,
                executable: false,
                ..DirEntry::default()
            },
            DirEntry {
                name: "beta.txt".to_owned(),
                kind: EntryKind::File,
                size: FileSize::from_bytes(20),
                modified: Modified::UNKNOWN,
                executable: false,
                ..DirEntry::default()
            },
        ],
    );

    panel.select(Some(1));
    panel.select_after_reload(&Panel::cursor_for_test(None, 0));

    assert_eq!(panel.selected(), Some(1));
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
