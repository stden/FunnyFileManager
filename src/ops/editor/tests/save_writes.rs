//! Purpose: The two ways a save is written, and a save that cannot be written.

use super::*;

/// The ordinary save writes a sibling temp file and renames it over the
/// target: `fs::write` truncates the original before the first new byte
/// lands, so a full disk mid-write would leave nothing but an empty file.
#[cfg(unix)]
#[test]
fn a_save_replaces_the_file_and_keeps_its_permissions() {
    use std::os::unix::fs::PermissionsExt;

    use crate::models::Editor;

    let dir = TempDir::new().expect("test setup");
    let path = dir.path().join("notes.txt");
    fs::write(&path, "old\n").expect("test setup");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).expect("test setup");

    let mut editor = Editor::from_content("notes.txt".to_owned(), path.clone(), "new\n");
    editor.save().expect("save");

    assert_eq!(fs::read_to_string(&path).unwrap(), "new\n");
    let mode = fs::metadata(&path).unwrap().permissions().mode();
    assert_eq!(
        mode & 0o777,
        0o640,
        "the original's mode survived the rename"
    );
}

/// A file with another name pointing at the same inode is rewritten in place
/// instead: renaming over it would break the other name, which is the whole
/// reason the two paths exist.
#[cfg(unix)]
#[test]
fn a_hard_linked_file_is_rewritten_in_place_so_the_other_name_survives() {
    use crate::models::Editor;

    let dir = TempDir::new().expect("test setup");
    let path = dir.path().join("notes.txt");
    let other = dir.path().join("also-notes.txt");
    fs::write(&path, "old\n").expect("test setup");
    fs::hard_link(&path, &other).expect("test setup");

    let mut editor = Editor::from_content("notes.txt".to_owned(), path.clone(), "new\n");
    editor.save().expect("save");

    assert_eq!(fs::read_to_string(&path).unwrap(), "new\n");
    assert_eq!(
        fs::read_to_string(&other).unwrap(),
        "new\n",
        "the other name sees the change, which a rename would have broken"
    );
}

/// A save into a directory that is gone reports the failure rather than
/// quietly marking the buffer clean.
#[test]
fn a_save_that_cannot_be_written_leaves_the_buffer_modified() {
    use crate::models::{EditState, Editor};

    let dir = TempDir::new().expect("test setup");
    let path = dir.path().join("gone/notes.txt");
    let mut editor = Editor::from_content("notes.txt".to_owned(), path, "new\n");
    editor.state = EditState::Modified;

    assert!(editor.save().is_err());
    assert_ne!(editor.state, EditState::Clean, "still unsaved");
}
