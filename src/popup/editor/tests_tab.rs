//! Unit tests: what the Tab key types — the file's own convention — and the
//! user-menu and associations files, whose format needs a real TAB.

use std::fs;

use tempfile::TempDir;

use crate::menu::MenuCommand;
use crate::popup::{process_popup_input, tests::app_with_editor};

use super::*;

fn press(app: &mut App, code: KeyCode) {
    assert!(process_popup_input(
        app,
        KeyEvent::new(code, KeyModifiers::NONE)
    ));
}

fn type_text(app: &mut App, text: &str) {
    for ch in text.chars() {
        press(app, KeyCode::Char(ch));
    }
}

fn lines(app: &App) -> Vec<String> {
    app.get_editor().expect("the editor is open").lines.clone()
}

/// A file that already uses TAB characters gets one: spaces would change
/// its format (a Makefile recipe, a TSV column, the user menu's separator).
#[test]
fn tab_types_a_tab_character_into_a_file_that_uses_them() {
    let temp = TempDir::new().expect("temporary directory");
    let mut app = app_with_editor("key\tvalue\n", temp.path().join("t.tsv"));

    press(&mut app, KeyCode::End);
    press(&mut app, KeyCode::Tab);

    assert_eq!(lines(&app), ["key\tvalue\t"]);
}

/// A file without TABs keeps the configured behaviour: spaces.
#[test]
fn tab_types_spaces_into_a_file_without_tabs() {
    let temp = TempDir::new().expect("temporary directory");
    let mut app = app_with_editor("plain\n", temp.path().join("t.txt"));

    press(&mut app, KeyCode::Tab);

    assert_eq!(lines(&app), ["    plain"]);
}

/// Open the user menu's command file through the menu, as "Edit user menu"
/// does, in a configuration directory of the test's own.
fn edit_command_file(temp: &TempDir, command: MenuCommand) -> App {
    let panel = crate::models::Panel::new(temp.path().to_path_buf()).expect("panel");
    let mut app = App::with_panels(panel.clone(), panel);
    app.settings =
        crate::models::settings::Settings::with_file(temp.path().join("config/settings.yaml"));
    crate::ops::menu::execute(&mut app, command);
    assert!(app.get_editor().is_some(), "test setup: the editor opened");
    app
}

/// "Label<TAB>command" is the file's format, and the built-in editor is the
/// only in-app way to edit it — yet Tab typed spaces, so the new line was
/// silently dropped and the user menu never showed it.
#[test]
fn a_user_menu_entry_typed_in_the_editor_reaches_the_user_menu() {
    let temp = TempDir::new().expect("temporary directory");
    let mut app = edit_command_file(&temp, MenuCommand::UserMenu);

    for _ in 0..lines(&app).len() {
        press(&mut app, KeyCode::Down);
    }
    press(&mut app, KeyCode::End);
    press(&mut app, KeyCode::Enter);
    type_text(&mut app, "Who am I");
    press(&mut app, KeyCode::Tab);
    type_text(&mut app, "id -un");
    press(&mut app, KeyCode::F(2));

    let saved = fs::read_to_string(temp.path().join("config/user-menu.txt")).expect("saved");
    assert!(saved.contains("Who am I\tid -un"), "{saved:?}");
    press(&mut app, KeyCode::Esc);
    crate::ops::menu::user::open(&mut app).expect("user menu");
    let crate::models::Overlay::MenuDialog(dialog) = &app.overlay else {
        panic!("the user menu is open");
    };
    let crate::models::MenuDialog::Choices { rows, .. } = dialog.as_ref() else {
        panic!("the user menu lists choices");
    };
    assert_eq!(rows[0].label, "Who am I");
}

/// The associations file has the same format. Tab types a TAB there even
/// when the user has deleted every line that had one.
#[test]
fn tab_types_a_tab_in_the_associations_file_even_with_no_tab_left_in_it() {
    let temp = TempDir::new().expect("temporary directory");
    fs::create_dir_all(temp.path().join("config")).expect("test setup");
    fs::write(temp.path().join("config/associations.txt"), "# empty\n").expect("test setup");
    let mut app = edit_command_file(&temp, MenuCommand::Associations);

    press(&mut app, KeyCode::End);
    press(&mut app, KeyCode::Enter);
    type_text(&mut app, "md");
    press(&mut app, KeyCode::Tab);
    type_text(&mut app, "cat {}");

    assert_eq!(lines(&app), ["# empty", "md\tcat {}"]);
}
