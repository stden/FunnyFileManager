//! Unit tests and shared test helpers for `models`.

use crate::models::BookmarkSlot;
use crate::models::EntryKind;
use crate::models::FileSize;
use crate::models::Modified;
use crate::models::{Bookmarks, FileName};
use crate::scroll::ScrollTop;
use std::fs;

use super::*;

use std::fs::File;

use std::io::Write;

use tempfile::TempDir;

#[cfg(test)]
mod editor;
#[cfg(test)]
mod newtypes;
#[cfg(test)]
mod panel_reload;

pub fn file_names(panel: &Panel) -> Vec<String> {
    panel
        .entries
        .iter()
        .filter(|entry| !entry.kind.is_dir() && entry.name != "..")
        .map(|entry| entry.name.clone())
        .collect()
}

pub fn editor_from(content: &str) -> Editor {
    Editor::from_content("test.txt".to_owned(), PathBuf::from("test.txt"), content)
}

#[cfg(windows)]
#[test]
fn test_windows_drive_paths_from_mask() {
    let drives = windows_drive_paths_from_mask((1 << 2) | (1 << 3) | (1 << 25));
    assert_eq!(
        drives,
        vec![
            PathBuf::from("C:\\"),
            PathBuf::from("D:\\"),
            PathBuf::from("Z:\\")
        ]
    );
}

#[test]
fn test_dir_entry_creation() {
    let entry = DirEntry {
        name: "test.txt".to_owned(),
        kind: EntryKind::File,
        size: FileSize::from_bytes(1024),
        modified: Modified::UNKNOWN,
        executable: false,
        ..DirEntry::default()
    };

    assert_eq!(entry.name, "test.txt");
    assert!(!entry.kind.is_dir());
    assert_eq!(entry.size, FileSize::from_bytes(1024));
}

#[test]
fn test_app_swap_panels() -> io::Result<()> {
    let temp_dir1 = TempDir::new().expect("test setup");
    let temp_dir2 = TempDir::new().expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel.path = temp_dir1.path().to_path_buf();
    app.right_panel.path = temp_dir2.path().to_path_buf();

    app.swap_panels();

    assert_eq!(app.left_panel.path, temp_dir2.path());
    assert_eq!(app.right_panel.path, temp_dir1.path());

    Ok(())
}

#[test]
fn test_get_drives() {
    let drives = App::get_drives();
    assert_ne!(drives.len(), 0);
}

#[test]
fn test_get_all_drives() {
    let drives = App::get_all_drives();
    assert_ne!(drives.len(), 0);
}

#[test]
fn test_drive_entry_display() {
    let local = DriveEntry::Local(PathBuf::from("/"));
    assert_eq!(local.display(), "/");

    let s3 = DriveEntry::S3Profile("default".to_owned());
    assert_eq!(s3.display(), "S3: default");
}

#[test]
fn test_dir_entry_clone() {
    let entry = DirEntry {
        name: "test.txt".to_owned(),
        kind: EntryKind::File,
        size: FileSize::from_bytes(1024),
        modified: Modified::UNKNOWN,
        executable: false,
        ..DirEntry::default()
    };

    let cloned = entry.clone();
    assert_eq!(cloned.name, entry.name);
    assert_eq!(cloned.kind.is_dir(), entry.kind.is_dir());
    assert_eq!(cloned.size, entry.size);
}

#[test]
fn test_viewer_default() {
    let viewer = Viewer::default();
    assert_eq!(viewer.title(), "");
    assert_eq!(viewer.lines.len(), 0);
    assert_eq!(viewer.scroll, ScrollTop::TOP);
}

#[test]
fn test_viewer_creation() {
    let viewer = Viewer {
        scroll: ScrollTop::TOP.down(5, usize::MAX),
        ..Viewer::report("test.txt", vec!["Line 1".to_owned(), "Line 2".to_owned()])
    };

    assert_eq!(viewer.title(), "test.txt");
    assert_eq!(viewer.lines.len(), 2);
    assert_eq!(viewer.scroll.lines(), 5);
}

#[test]
fn test_uncovered_models_methods() -> io::Result<()> {
    assert!(BookmarkSlot::new(10).is_err());
    assert!(BookmarkSlot::from_usize(11).is_err());

    let slot = BookmarkSlot::new(5).unwrap();
    assert_eq!(slot.as_u8(), 5);
    assert_eq!(slot.as_usize(), 5);

    let mut app = App::new()?;

    let overlay = app.get_overlay_mut();
    assert!(matches!(overlay, Overlay::None));

    assert!(app.get_viewer_mut().is_none());
    assert!(app.get_command_line_mut().is_none());
    assert!(app.get_new_file_input_mut().is_none());

    app.set_overlay(Overlay::Viewer(Viewer::default()));
    assert!(app.get_viewer_mut().is_some());

    app.set_overlay(Overlay::CommandLine(CommandLine::default()));
    assert!(app.get_command_line_mut().is_some());

    app.set_overlay(Overlay::NewFileInput(String::new().into()));
    assert!(app.get_new_file_input_mut().is_some());

    Ok(())
}

#[test]
fn test_command_line_overlay_exposes_its_buffer() {
    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::CommandLine("cd /tmp".into()));

    // The accessors keep their old shape so existing call sites are unaffected.
    assert_eq!(app.get_command_line(), Some("cd /tmp"));

    app.get_command_line_mut()
        .expect("command line")
        .insert_char('/');
    assert_eq!(app.get_command_line(), Some("cd /tmp/"));
}
