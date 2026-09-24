//! Unit tests: when F2 and the exit dialog's Save write the file, and when
//! they must not.

use std::fs;

use tempfile::TempDir;

use crate::popup::{process_popup_input, tests::app_with_editor};

use super::*;

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

// ─── a file changed on disk since it was opened ─────────────────────────────

/// F4 on `name` holding `content`, through the real loader, so the editor
/// knows what the disk held when it opened.
fn edit_file(temp: &TempDir, name: &str, content: &str) -> App {
    fs::write(temp.path().join(name), content).expect("test setup");
    let panel = || crate::models::Panel::new(temp.path().to_path_buf()).expect("panel");
    let mut app = App::with_panels(panel(), panel());
    let index = app
        .left_panel
        .entries
        .iter()
        .position(|entry| entry.name == name)
        .expect("the file is listed");
    app.left_panel.select(Some(index));
    crate::editor::handle_edit_builtin(&mut app).expect("F4");
    assert!(app.get_editor().is_some(), "test setup: the editor opened");
    app
}

/// Type `text` at the start of the first line.
fn type_at_start(app: &mut App, text: &str) {
    press(app, KeyCode::Home);
    for ch in text.chars() {
        press(app, KeyCode::Char(ch));
    }
}

/// Another program wrote the file while it was open. F2 used to replace that
/// write in silence; now it asks, and No keeps the other program's text.
#[test]
fn f2_asks_before_overwriting_a_file_changed_on_disk_and_no_keeps_it() {
    let temp = TempDir::new().expect("temporary directory");
    let path = temp.path().join("efile.txt");
    let mut app = edit_file(&temp, "efile.txt", "original\n");
    type_at_start(&mut app, "EDIT ");
    fs::write(&path, "external change\n").expect("the other program's write");

    press(&mut app, KeyCode::F(2));

    assert_eq!(
        fs::read_to_string(&path).expect("file"),
        "external change\n",
        "F2 must ask before replacing the other write"
    );
    assert!(
        !editor(&app).accepts_text(),
        "the question owns the keyboard"
    );

    press(&mut app, KeyCode::Char('n'));

    assert_eq!(
        fs::read_to_string(&path).expect("file"),
        "external change\n"
    );
    assert_eq!(editor(&app).state, EditState::Modified, "the edit is kept");
    assert_eq!(editor(&app).lines, ["EDIT original"]);
}

/// Yes is an explicit decision to replace the other program's change.
#[test]
fn f2_overwrites_a_changed_file_once_the_user_says_so() {
    let temp = TempDir::new().expect("temporary directory");
    let path = temp.path().join("efile.txt");
    let mut app = edit_file(&temp, "efile.txt", "original\n");
    type_at_start(&mut app, "EDIT ");
    fs::write(&path, "external change\n").expect("the other program's write");

    press(&mut app, KeyCode::F(2));
    press(&mut app, KeyCode::Char('y'));

    assert_eq!(fs::read_to_string(&path).expect("file"), "EDIT original\n");
    assert_eq!(editor(&app).state, EditState::Clean);
}

/// The exit dialog's Save is the same write, so it asks the same question —
/// and the editor closes only once the file is written.
#[test]
fn the_exit_dialogs_save_asks_too_and_closes_after_the_overwrite() {
    let temp = TempDir::new().expect("temporary directory");
    let path = temp.path().join("efile.txt");
    let mut app = edit_file(&temp, "efile.txt", "original\n");
    type_at_start(&mut app, "EDIT ");
    fs::write(&path, "external change\n").expect("the other program's write");

    press(&mut app, KeyCode::Esc); // "File has been modified. Save?"
    press(&mut app, KeyCode::Char('y'));

    assert!(app.get_editor().is_some(), "still asking, still open");
    assert_eq!(
        fs::read_to_string(&path).expect("file"),
        "external change\n"
    );

    press(&mut app, KeyCode::Char('y'));

    assert!(app.get_editor().is_none(), "saved, then closed");
    assert_eq!(fs::read_to_string(&path).expect("file"), "EDIT original\n");
}

/// The editor's own saves are not "another program": a second F2 after a
/// first one writes straight through.
#[test]
fn saving_twice_in_a_row_does_not_ask() {
    let temp = TempDir::new().expect("temporary directory");
    let path = temp.path().join("efile.txt");
    let mut app = edit_file(&temp, "efile.txt", "original\n");

    type_at_start(&mut app, "1");
    press(&mut app, KeyCode::F(2));
    type_at_start(&mut app, "2");
    press(&mut app, KeyCode::F(2));

    assert_eq!(fs::read_to_string(&path).expect("file"), "21original\n");
    assert_eq!(editor(&app).state, EditState::Clean);
}

/// The question opens on Cancel, so a reflexive Enter keeps the other
/// program's write; moving to Overwrite first is a deliberate choice.
#[test]
fn enter_cancels_until_the_cursor_is_moved_to_overwrite() {
    let temp = TempDir::new().expect("temporary directory");
    let path = temp.path().join("efile.txt");
    let mut app = edit_file(&temp, "efile.txt", "original\n");
    type_at_start(&mut app, "EDIT ");
    fs::write(&path, "external change\n").expect("the other program's write");

    press(&mut app, KeyCode::F(2));
    press(&mut app, KeyCode::Enter);
    assert_eq!(
        fs::read_to_string(&path).expect("file"),
        "external change\n"
    );
    assert_eq!(editor(&app).state, EditState::Modified);

    press(&mut app, KeyCode::F(2));
    press(&mut app, KeyCode::Left);
    press(&mut app, KeyCode::Enter);
    assert_eq!(fs::read_to_string(&path).expect("file"), "EDIT original\n");
}
