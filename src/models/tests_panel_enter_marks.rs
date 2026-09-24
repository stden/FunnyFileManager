//! Unit tests: panel enter marks (split out to keep files under 500 lines).

use crate::models::FileName;
use std::fs;

use super::*;

use tempfile::TempDir;

mod tests_enter;
mod tests_marks;
mod tests_navigate_to;
mod tests_sorting;

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

/// `DirEntry` display for directories vs files
#[test]
fn test_dir_entry_properties() {
    let dir = DirEntry {
        name: "mydir".to_owned(),
        kind: EntryKind::Directory,
        size: FileSize::from_bytes(0),
        modified: Modified::UNKNOWN,
        executable: false,
        ..DirEntry::default()
    };
    assert!(dir.kind.is_dir());
    assert_eq!(dir.size, FileSize::from_bytes(0));

    let file = DirEntry {
        name: "myfile.rs".to_owned(),
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
