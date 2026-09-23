//! Purpose: unit tests for the cursor after reloads, `enter` and out-of-range selects.

use super::*;

#[test]
fn test_panel_load_entries_with_selected_name_edge_case() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    // Create files
    File::create(temp_dir.path().join("file1.txt")).expect("test setup");
    File::create(temp_dir.path().join("file2.txt")).expect("test setup");

    // Select file2.txt
    panel.load_entries().expect("test setup");
    for (i, entry) in panel.entries.iter().enumerate() {
        if entry.name == "file2.txt" {
            panel.select(Some(i));
            break;
        }
    }

    // Reload should preserve selection of file2.txt
    panel.load_entries().expect("test setup");

    // Should still have file2.txt selected
    let idx = panel.selected().expect("selection");
    assert_eq!(panel.entries[idx].name, "file2.txt");

    // Now delete file2.txt and reload — the cursor holds its row rather than
    // springing back to the top, which is how Far leaves it on the neighbour.
    let row = panel.selected().expect("selection");
    fs::remove_file(temp_dir.path().join("file2.txt")).expect("test setup");
    panel.load_entries().expect("test setup");

    assert_eq!(panel.selected(), Some(row.min(panel.entries.len() - 1)));

    Ok(())
}

#[test]
fn test_panel_ensure_selection_corner_case() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    // Test empty entries with no selection
    panel.entries.clear();
    panel.select(None);
    panel.ensure_selection();
    assert_eq!(panel.selected(), None); // Should remain None for empty entries

    Ok(())
}

#[test]
fn test_enter_parent_selects_previous_dir() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let subdir = temp_dir.path().join("target_dir");
    fs::create_dir(&subdir)?;
    // Create siblings so target_dir isn't the only entry
    fs::create_dir(temp_dir.path().join("aaa"))?;
    fs::create_dir(temp_dir.path().join("zzz"))?;

    let mut panel = Panel::new(subdir)?;
    // Go up to parent via ".."
    panel.select(Some(0)); // ".." is always first
    panel.enter()?;

    // Cursor should now be on "target_dir"
    let selected = panel.get_selected_entry().expect("should have selection");
    assert_eq!(selected.name, "target_dir");

    Ok(())
}

#[test]
fn a_selection_past_the_end_becomes_no_selection() {
    // It used to be each caller's job to check, including one written-out
    // `filter(|i| *i < entries.len())`. An index naming no row would render
    // nothing while `get_selected_entry` reported "nothing selected" — a bug
    // wearing an ordinary state's clothes.
    let mut panel = Panel::from_entries(
        PathBuf::from("/tmp"),
        vec![DirEntry {
            name: "only.txt".to_owned(),
            kind: EntryKind::File,
            size: FileSize::from_bytes(0),
            modified: Modified::UNKNOWN,
            executable: false,
            ..DirEntry::default()
        }],
    );
    assert_eq!(panel.selected(), Some(0));

    panel.select(Some(99));
    assert_eq!(panel.selected(), None);

    panel.select(Some(0));
    assert_eq!(panel.selected(), Some(0));

    panel.select(None);
    assert_eq!(panel.selected(), None);
}

/// `enter()` guards its index a second time, independently of `select`'s own
/// bounds check: `entries` is a public field, so a caller that mutates it
/// directly (rather than going through `select`/`load_entries`) can leave the
/// cursor pointing past the new end. `enter` must return `Ok(())` rather than
/// panicking on the stale index.
#[test]
fn enter_is_a_no_op_when_the_selected_index_is_stale_after_entries_shrink() {
    let mut panel = Panel::from_entries(
        PathBuf::from("/tmp"),
        vec![
            DirEntry {
                name: "a".to_owned(),
                kind: EntryKind::Directory,
                size: FileSize::from_bytes(0),
                modified: Modified::UNKNOWN,
                executable: false,
                ..DirEntry::default()
            },
            DirEntry {
                name: "b".to_owned(),
                kind: EntryKind::Directory,
                size: FileSize::from_bytes(0),
                modified: Modified::UNKNOWN,
                executable: false,
                ..DirEntry::default()
            },
        ],
    );
    panel.select(Some(1));
    // Bypass `select`'s own bounds check by mutating the public field
    // directly — exactly the stale-index state `enter`'s second guard exists
    // to survive.
    panel.entries.truncate(1);

    assert!(panel.enter().is_ok(), "a stale index must not panic");
}

#[test]
fn an_empty_panel_can_select_nothing_at_all() {
    let mut panel = Panel::from_entries(PathBuf::from("/tmp"), Vec::new());
    assert_eq!(panel.selected(), None);
    panel.select(Some(0));
    assert_eq!(panel.selected(), None, "there is no row 0 to highlight");
    panel.ensure_selection();
    assert_eq!(panel.selected(), None);
}
