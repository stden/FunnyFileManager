//! Unit tests: panel edge cases (split out to keep files under 500 lines).
#![allow(unused_imports)]

use crate::models::FileName;
use crate::models::Modified;
use std::fs;

use super::*;

use std::fs::File;

use tempfile::TempDir;

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
fn test_panel_sort_modes_with_mixed_case() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    // Create files with mixed case names
    File::create(temp_dir.path().join("UPPERCASE.txt")).expect("test setup");
    File::create(temp_dir.path().join("lowercase.txt")).expect("test setup");
    File::create(temp_dir.path().join("MixedCase.txt")).expect("test setup");
    fs::create_dir(temp_dir.path().join("Directory")).expect("test setup");

    // Test that directories come before files in all sort modes
    panel.sort_mode = SortMode::Name;
    panel.load_entries().expect("test setup");

    // Find positions of directory and files
    let dir_pos = panel
        .entries
        .iter()
        .position(|e| e.name == "Directory" && e.kind.is_dir());
    let file_positions: Vec<_> = panel
        .entries
        .iter()
        .enumerate()
        .filter(|(_, e)| !e.kind.is_dir() && e.name != "..")
        .map(|(i, _)| i)
        .collect();

    let dir_idx = dir_pos.expect("Directory entry");
    for file_idx in &file_positions {
        assert!(dir_idx < *file_idx, "Directory should come before files");
    }

    Ok(())
}

#[test]
fn test_quick_search_find_first_matching() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    File::create(temp_dir.path().join("alpha.txt")).expect("test setup");
    File::create(temp_dir.path().join("beta.txt")).expect("test setup");
    File::create(temp_dir.path().join("gamma.txt")).expect("test setup");
    File::create(temp_dir.path().join("Alpha_upper.txt")).expect("test setup");

    let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    // Find first entry starting with "a" (case-insensitive)
    assert!(panel.find_first_matching("a"));
    let selected = panel.get_selected_entry().unwrap();
    assert!(selected.name.to_lowercase().starts_with('a'));

    // Find first entry starting with "b"
    assert!(panel.find_first_matching("b"));
    let selected = panel.get_selected_entry().unwrap();
    assert_eq!(selected.name, "beta.txt");

    // Non-existent prefix
    assert!(!panel.find_first_matching("z"));

    // Empty prefix
    assert!(!panel.find_first_matching(""));

    Ok(())
}

#[test]
fn test_quick_search_find_next_matching() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    File::create(temp_dir.path().join("apple.txt")).expect("test setup");
    File::create(temp_dir.path().join("apricot.txt")).expect("test setup");
    File::create(temp_dir.path().join("avocado.txt")).expect("test setup");
    File::create(temp_dir.path().join("banana.txt")).expect("test setup");

    let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    // Find first "a" entry
    assert!(panel.find_first_matching("a"));
    let first_match = panel.get_selected_entry().unwrap().name.clone();

    // Find next "a" entry - should cycle to a different one
    assert!(panel.find_next_matching("a"));
    let second_match = panel.get_selected_entry().unwrap().name.clone();
    assert_ne!(first_match, second_match);
    assert!(second_match.to_lowercase().starts_with('a'));

    Ok(())
}

#[test]
fn test_quick_search_empty_panel() {
    let mut panel = Panel::from_entries(PathBuf::new(), Vec::new());

    assert!(!panel.find_first_matching("a"));
    assert!(!panel.find_next_matching("a"));
}

#[test]
fn test_app_quick_search_field() -> io::Result<()> {
    let app = App::new().expect("test setup");
    assert!(app.quick_search.query().is_none());
    Ok(())
}

// -----------------------------------------------------------------------
// find_next_matching – additional branch coverage
// -----------------------------------------------------------------------

/// When there is exactly one matching entry, `find_next_matching` wraps
/// all the way around and returns true (selecting the same entry again).
#[test]
fn test_find_next_matching_single_match_wraps_to_itself() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    File::create(temp_dir.path().join("unique.txt")).expect("test setup");
    File::create(temp_dir.path().join("other.txt")).expect("test setup");

    let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    // Land on "unique.txt" first.
    assert!(panel.find_first_matching("unique"));
    let first_idx = panel.selected().expect("should be selected");

    // find_next_matching should wrap around and find the same entry.
    let found = panel.find_next_matching("unique");
    assert!(found, "should find the only match (wrapping)");
    assert_eq!(
        panel.selected(),
        Some(first_idx),
        "should wrap back to the same entry"
    );

    Ok(())
}

/// `find_next_matching` returns false when the prefix is empty.
#[test]
fn test_find_next_matching_empty_prefix_returns_false() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    File::create(temp_dir.path().join("file.txt")).expect("test setup");

    let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    let found = panel.find_next_matching("");
    assert!(!found, "empty prefix should return false");

    Ok(())
}

/// `find_next_matching` returns false when no entry matches.
#[test]
fn test_find_next_matching_no_match_returns_false() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    File::create(temp_dir.path().join("alpha.txt")).expect("test setup");

    let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    let found = panel.find_next_matching("zzz");
    assert!(!found, "non-matching prefix should return false");

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

/// `cycle_sort_mode` cycles Name -> Extension -> Size -> Name.
#[test]
fn test_panel_cycle_sort_mode_full_cycle() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    assert_eq!(
        panel.sort_mode,
        SortMode::Name,
        "initial mode should be Name"
    );

    panel.cycle_sort_mode();
    assert_eq!(panel.sort_mode, SortMode::Extension);

    panel.cycle_sort_mode();
    assert_eq!(panel.sort_mode, SortMode::Size);

    panel.cycle_sort_mode();
    assert_eq!(panel.sort_mode, SortMode::Name, "should wrap back to Name");

    Ok(())
}

/// `App::swap_panels` exchanges the paths of both panels.
#[test]
fn test_app_swap_panels_exchanges_paths() -> io::Result<()> {
    let temp_dir1 = TempDir::new().expect("test setup");
    let temp_dir2 = TempDir::new().expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir1.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir2.path().to_path_buf()).expect("test setup");

    let left_path_before = app.left_panel.path.clone();
    let right_path_before = app.right_panel.path.clone();

    app.swap_panels();

    assert_eq!(
        app.left_panel.path, right_path_before,
        "left should now hold right's old path"
    );
    assert_eq!(
        app.right_panel.path, left_path_before,
        "right should now hold left's old path"
    );

    Ok(())
}

/// `App::get_all_drives` returns at least one drive (the root on Unix or C:\ on Windows).
#[test]
fn test_get_all_drives_returns_at_least_one_drive() {
    let drives = App::get_all_drives();
    assert!(
        !drives.is_empty(),
        "get_all_drives should return at least one entry"
    );

    // The first drive should be a Local variant.
    assert!(
        matches!(&drives[0], crate::models::DriveEntry::Local(_)),
        "first drive should be a Local entry"
    );
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
            name: "only.txt".to_string(),
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
                name: "a".to_string(),
                kind: EntryKind::Directory,
                size: FileSize::from_bytes(0),
                modified: Modified::UNKNOWN,
                executable: false,
                ..DirEntry::default()
            },
            DirEntry {
                name: "b".to_string(),
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
