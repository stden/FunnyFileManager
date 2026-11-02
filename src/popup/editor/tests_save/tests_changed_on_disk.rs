//! Purpose: saving a file that changed on disk since it was opened — F2 and the exit dialog's Save ask before overwriting it, and ask once.

use super::*;

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
