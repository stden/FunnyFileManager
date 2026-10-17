//! Unit tests: panel edge cases.

use crate::models::FileName;
use crate::models::Modified;
use std::fs;

use super::*;

use std::fs::File;

use tempfile::TempDir;

mod tests_marks;
mod tests_quick_search;
mod tests_selection;

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
        matches!(&drives[0], DriveEntry::Local(_)),
        "first drive should be a Local entry"
    );
}
