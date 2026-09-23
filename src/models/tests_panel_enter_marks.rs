//! Unit tests: panel enter marks (split out to keep files under 500 lines).
#![allow(unused_imports)]

use crate::models::EntryKind;
use crate::models::FileName;
use crate::models::FileSize;
use crate::models::Modified;
use std::fs;

use super::*;

use std::io::Write;

use tempfile::TempDir;

/// enter("..") from root: `prev_dir_name` is None, fallback selects first entry
#[test]
fn test_enter_parent_from_root_fallback() -> io::Result<()> {
    let root = if cfg!(windows) { "C:\\" } else { "/" };
    let mut panel = Panel::new(PathBuf::from(root))?;

    // ".." should be the first entry (or root has no "..")
    if let Some(dotdot_pos) = panel.entries.iter().position(|e| e.name == "..") {
        panel.select(Some(dotdot_pos));
        panel.enter()?;
        // Should still be at root with a valid selection
        assert!(panel.selected().is_some());
    }

    Ok(())
}

/// `enter()` on a file entry is a no-op (returns Ok without changing path)
#[test]
fn test_enter_file_is_noop() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    fs::write(temp_dir.path().join("file.txt"), "data")?;

    let mut panel = Panel::new(temp_dir.path().to_path_buf())?;
    let original_path = panel.path.clone();

    // Select the file
    for (i, entry) in panel.entries.iter().enumerate() {
        if entry.name == "file.txt" {
            panel.select(Some(i));
            break;
        }
    }

    panel.enter()?;
    assert_eq!(panel.path, original_path, "path should not change for file");

    Ok(())
}

/// `enter()` with no selection is a no-op
#[test]
fn test_enter_no_selection_is_noop() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut panel = Panel::new(temp_dir.path().to_path_buf())?;
    panel.select(None);
    let original_path = panel.path.clone();

    panel.enter()?;
    assert_eq!(panel.path, original_path);

    Ok(())
}

/// `enter()` into a subdirectory clears marks and selects first entry
#[test]
fn test_enter_subdir_clears_marks() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let subdir = temp_dir.path().join("sub");
    fs::create_dir(&subdir)?;
    fs::write(temp_dir.path().join("file.txt"), "data")?;

    let mut panel = Panel::new(temp_dir.path().to_path_buf())?;
    panel.marked.insert(FileName::from("file.txt"));

    // Select the subdirectory
    for (i, entry) in panel.entries.iter().enumerate() {
        if entry.name == "sub" && entry.kind.is_dir() {
            panel.select(Some(i));
            break;
        }
    }

    panel.enter()?;
    assert!(panel.marked.is_empty(), "marks should be cleared");
    assert!(panel.path.ends_with("sub"));

    Ok(())
}

/// `cycle_sort_mode` cycles through all three modes
#[test]
fn test_cycle_sort_mode_all_three() {
    let temp_dir = TempDir::new().expect("temp dir");
    let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("panel");

    assert_eq!(panel.sort_mode, SortMode::Name);
    panel.cycle_sort_mode();
    assert_eq!(panel.sort_mode, SortMode::Extension);
    panel.cycle_sort_mode();
    assert_eq!(panel.sort_mode, SortMode::Size);
    panel.cycle_sort_mode();
    assert_eq!(panel.sort_mode, SortMode::Name);
}

/// `ensure_selection` promotes None to Some(0) when entries exist
#[test]
fn test_ensure_selection_none_becomes_zero() {
    let temp_dir = TempDir::new().expect("temp dir");
    fs::write(temp_dir.path().join("a.txt"), "a").expect("write");
    let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("panel");
    panel.select(None);

    panel.ensure_selection();
    assert_eq!(panel.selected(), Some(0));
}

/// `ensure_selection` is no-op when entries are empty
#[test]
fn test_ensure_selection_empty_entries_noop() {
    let temp_dir = TempDir::new().expect("temp dir");
    let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("panel");
    panel.entries.clear();
    panel.select(None);

    panel.ensure_selection();
    assert_eq!(panel.selected(), None);
}

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

/// `DirEntry` display for directories vs files
#[test]
fn test_dir_entry_properties() {
    let dir = DirEntry {
        name: "mydir".to_string(),
        kind: EntryKind::Directory,
        size: FileSize::from_bytes(0),
        modified: Modified::UNKNOWN,
        executable: false,
        ..DirEntry::default()
    };
    assert!(dir.kind.is_dir());
    assert_eq!(dir.size, FileSize::from_bytes(0));

    let file = DirEntry {
        name: "myfile.rs".to_string(),
        kind: EntryKind::File,
        size: FileSize::from_bytes(4096),
        modified: Modified::UNKNOWN,
        executable: false,
        ..DirEntry::default()
    };
    assert!(!file.kind.is_dir());
    assert_eq!(file.size, FileSize::from_bytes(4096));
}

/// `Panel::new` on nonexistent path returns error
#[test]
fn test_panel_new_nonexistent_path() {
    let result = Panel::new(PathBuf::from("/nonexistent_path_12345"));
    assert!(result.is_err());
}

/// enter("..") fallback when previous dir name not found in parent entries
#[test]
fn test_enter_parent_prev_dir_deleted_falls_back_to_first() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let child = temp_dir.path().join("child");
    fs::create_dir(&child)?;
    let mut panel = Panel::new(child.clone())?;

    // Delete the child dir so it won't appear when we go up
    fs::remove_dir(&child)?;

    // Select ".."
    if let Some(pos) = panel.entries.iter().position(|e| e.name == "..") {
        panel.select(Some(pos));
        panel.enter()?;
        // Should fall back to selecting first entry (line 223)
        assert_eq!(panel.selected(), Some(0));
    }
    Ok(())
}

// Kill: delete ! in (!e.kind.is_dir(), ext, ...) in Extension sort
#[test]
fn test_extension_sort_dirs_before_files() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    fs::create_dir(temp_dir.path().join("zdir"))?;
    fs::write(temp_dir.path().join("aaa.txt"), "a")?;

    let mut panel = Panel::new(temp_dir.path().to_path_buf())?;
    panel.sort_mode = SortMode::Extension;
    panel.load_entries()?;

    // Find positions: ".." first, then dirs, then files
    let dir_pos = panel.entries.iter().position(|e| e.name == "zdir").unwrap();
    let file_pos = panel
        .entries
        .iter()
        .position(|e| e.name == "aaa.txt")
        .unwrap();
    assert!(
        dir_pos < file_pos,
        "directories must sort before files in Extension mode: dir@{dir_pos} file@{file_pos}"
    );

    Ok(())
}

// Kill: delete match arm (true, false) => Less in Size sort
// Kill: delete match arm (false, true) => Greater in Size sort
#[test]
fn test_size_sort_dirs_before_files() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    // Create multiple dirs and files interspersed alphabetically
    // so the sort must encounter both (dir,file) and (file,dir) comparisons
    fs::create_dir(temp_dir.path().join("adir"))?;
    fs::create_dir(temp_dir.path().join("mdir"))?;
    fs::create_dir(temp_dir.path().join("zdir"))?;
    fs::write(temp_dir.path().join("bfile.bin"), vec![0u8; 10000])?;
    fs::write(temp_dir.path().join("nfile.txt"), vec![0u8; 5000])?;
    fs::write(temp_dir.path().join("yfile.dat"), "x")?;

    let mut panel = Panel::new(temp_dir.path().to_path_buf())?;
    panel.sort_mode = SortMode::Size;
    panel.load_entries()?;

    assert_directories_before_files(&panel);
    assert_files_descending(&panel);

    Ok(())
}

// Kill: replace App::get_drives -> Vec<PathBuf> with vec![] or vec![Default::default()]
#[test]
fn test_get_drives_returns_valid_paths() {
    let drives = App::get_drives();
    assert!(
        !drives.is_empty(),
        "get_drives must return at least one drive"
    );
    for drive in &drives {
        assert!(drive.exists(), "drive path must exist: {}", drive.display());
        assert!(
            drive.is_dir(),
            "drive path must be a directory: {}",
            drive.display()
        );
    }
}

// -----------------------------------------------------------------------
// navigate_to: path and marks move together, failures roll back
// -----------------------------------------------------------------------

/// Marks are bare names, so they must never outlive the path they were taken in.
#[test]
fn test_navigate_to_clears_marks() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let other = temp_dir.path().join("other");
    fs::create_dir(&other)?;
    fs::write(temp_dir.path().join("tmp"), "here")?;
    fs::write(other.join("tmp"), "a different file with the same name")?;

    let mut panel = Panel::new(temp_dir.path().to_path_buf())?;
    panel.marked.insert(FileName::from("tmp"));

    panel.navigate_to(other.clone())?;

    assert_eq!(panel.path, other);
    assert!(
        panel.marked.is_empty(),
        "a name-keyed mark must not re-bind to a same-named file elsewhere"
    );
    assert!(panel.selected().is_some());

    Ok(())
}

/// A failed load must leave the panel on its previous, readable directory —
/// path, marks and selection included.
#[test]
fn test_navigate_to_failure_restores_previous_directory() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    fs::write(temp_dir.path().join("a.txt"), "a")?;

    let mut panel = Panel::new(temp_dir.path().to_path_buf())?;
    panel.marked.insert(FileName::from("a.txt"));
    let selection_before = panel.selected();

    let error = panel
        .navigate_to(temp_dir.path().join("does_not_exist"))
        .expect_err("missing directory must fail");

    assert_eq!(error.kind(), io::ErrorKind::NotFound);
    assert_eq!(panel.path, temp_dir.path());
    assert!(panel.entries.iter().any(|entry| entry.name == "a.txt"));
    assert_eq!(panel.selected(), selection_before);
    assert!(
        panel.marked.contains("a.txt"),
        "marks belong to the directory we never left"
    );

    Ok(())
}

/// `enter()` used to commit the path *before* the fallible read, stranding the
/// panel on a directory it had no listing for (unreadable dir, removed dir).
#[test]
fn test_enter_vanished_dir_leaves_panel_untouched() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    fs::write(temp_dir.path().join("a.txt"), "a")?;
    let gone = temp_dir.path().join("gone");
    fs::create_dir(&gone)?;

    let mut panel = Panel::new(temp_dir.path().to_path_buf())?;
    panel.marked.insert(FileName::from("a.txt"));
    let index = panel
        .entries
        .iter()
        .position(|entry| entry.name == "gone")
        .expect("test setup");
    panel.select(Some(index));

    // Removed after the listing was taken: the entry is still on screen.
    fs::remove_dir(&gone)?;
    let result = panel.enter();

    assert!(result.is_err(), "entering a missing directory must fail");
    assert_eq!(panel.path, temp_dir.path(), "panel must not move");
    assert!(panel.entries.iter().any(|entry| entry.name == "a.txt"));
    assert!(
        panel.entries.iter().all(|entry| entry.name != "gone"),
        "the listing is reloaded, so the removed entry is gone"
    );
    let selected = panel.selected().expect("selection must stay valid");
    assert!(selected < panel.entries.len());
    assert!(panel.marked.contains("a.txt"));

    Ok(())
}

/// Same guarantee via the permission path (skipped when running as root, which
/// ignores the mode bits).
#[test]
#[cfg(unix)]
fn test_enter_unreadable_dir_leaves_panel_untouched() -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let temp_dir = TempDir::new()?;
    let locked = temp_dir.path().join("locked");
    fs::create_dir(&locked)?;
    // Listing the parent only needs search permission, so the entry shows up as
    // a directory and read_dir on it then fails with EACCES.
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o000))?;

    let mut panel = Panel::new(temp_dir.path().to_path_buf())?;
    let index = panel
        .entries
        .iter()
        .position(|entry| entry.name == "locked")
        .expect("test setup");
    panel.select(Some(index));

    let result = panel.enter();
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o700))?;

    if result.is_ok() {
        return Ok(()); // root bypasses the mode bits — nothing to assert
    }
    assert_eq!(panel.path, temp_dir.path());
    assert_eq!(panel.selected(), Some(index));

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

fn assert_directories_before_files(panel: &Panel) {
    // All directories must appear before all files
    let all_dir_positions: Vec<usize> = panel
        .entries
        .iter()
        .enumerate()
        .filter(|(_, e)| e.kind.is_dir() && e.name != "..")
        .map(|(i, _)| i)
        .collect();
    let all_file_positions: Vec<usize> = panel
        .entries
        .iter()
        .enumerate()
        .filter(|(_, e)| !e.kind.is_dir())
        .map(|(i, _)| i)
        .collect();

    let max_dir = all_dir_positions.iter().max().copied().unwrap_or(0);
    let min_file = all_file_positions
        .iter()
        .min()
        .copied()
        .unwrap_or(usize::MAX);
    assert!(
        max_dir < min_file,
        "all dirs must come before all files: max_dir_pos={max_dir} min_file_pos={min_file}"
    );
}

fn assert_files_descending(panel: &Panel) {
    // Files should be ordered by size descending
    let file_sizes: Vec<u64> = panel
        .entries
        .iter()
        .filter(|e| !e.kind.is_dir())
        .map(|e| e.size.bytes())
        .collect();
    for w in file_sizes.windows(2) {
        assert!(
            w[0] >= w[1],
            "files must be sorted by size descending: {} < {}",
            w[0],
            w[1]
        );
    }
}
