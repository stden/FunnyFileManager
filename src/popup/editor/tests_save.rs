//! Unit tests: when F2 and the exit dialog's Save write the file, and when
//! they must not.

use std::fs;

use tempfile::TempDir;

use crate::models::EditState;
use crate::popup::{process_popup_input, tests::app_with_editor};

use super::*;

mod tests_changed_on_disk;

fn press(app: &mut App, code: KeyCode) {
    assert!(process_popup_input(
        app,
        KeyEvent::new(code, KeyModifiers::NONE)
    ));
}

fn editor(app: &App) -> &crate::models::Editor {
    app.get_editor().expect("the editor is open")
}

/// F2 with nothing edited has nothing to write. It used to rewrite the file
/// anyway, which is how an untouched binary lost bytes and an untouched mixed
/// file lost its CRLFs. The buffer here deliberately differs from the disk,
/// so any write at all would show.
#[test]
fn f2_on_an_unmodified_buffer_leaves_the_file_alone() {
    let temp = TempDir::new().expect("temporary directory");
    let path = temp.path().join("text.txt");
    fs::write(&path, "on disk\n").expect("test setup");
    let mut app = app_with_editor("in the buffer\n", path.clone());

    press(&mut app, KeyCode::F(2));

    assert_eq!(fs::read_to_string(&path).expect("file"), "on disk\n");
    assert_eq!(editor(&app).state, EditState::Clean);
    assert_eq!(
        editor(&app).notice.as_deref(),
        Some("No changes to save"),
        "the key is answered, not ignored"
    );
}

/// The same thing on a real file: an atomic save replaces the inode, so an
/// unchanged inode proves nothing was written.
#[cfg(unix)]
#[test]
fn f2_on_an_unmodified_file_keeps_its_inode() {
    use std::os::unix::fs::MetadataExt;

    let temp = TempDir::new().expect("temporary directory");
    let path = temp.path().join("text.txt");
    fs::write(&path, "same\n").expect("test setup");
    let inode = fs::metadata(&path).expect("metadata").ino();
    let mut app = app_with_editor("same\n", path.clone());

    press(&mut app, KeyCode::F(2));

    assert_eq!(fs::metadata(&path).expect("metadata").ino(), inode);
}

/// An edit is still saved by F2, of course.
#[test]
fn f2_on_an_edited_buffer_writes_it() {
    let temp = TempDir::new().expect("temporary directory");
    let path = temp.path().join("text.txt");
    fs::write(&path, "one\n").expect("test setup");
    let mut app = app_with_editor("one\n", path.clone());

    press(&mut app, KeyCode::End);
    press(&mut app, KeyCode::Char('!'));
    press(&mut app, KeyCode::F(2));

    assert_eq!(fs::read_to_string(&path).expect("file"), "one!\n");
    assert_eq!(editor(&app).state, EditState::Clean);
}
