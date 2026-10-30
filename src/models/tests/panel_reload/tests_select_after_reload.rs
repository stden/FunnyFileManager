//! Purpose: unit tests for where the cursor lands after a reload.

use super::*;

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
