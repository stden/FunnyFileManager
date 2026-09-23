//! Purpose: the Menu family: keyless rows, Enter on an open menu row, and Enter on an associated file.

use super::*;

#[test]
fn each_keyless_row_takes_the_same_road() {
    // The rows Far gives no key each reach a working command now: the two that
    // are plain text files open the editor, the rest open their dialog.
    let cases: [(MenuCommand, Opens); 4] = [
        (MenuCommand::Associations, |o| {
            matches!(o, Overlay::Editor(_))
        }),
        (MenuCommand::UserMenu, |o| matches!(o, Overlay::Editor(_))),
        (MenuCommand::Shortcuts, |o| {
            matches!(o, Overlay::MenuDialog(_))
        }),
        (MenuCommand::Devices, |o| {
            matches!(o, Overlay::MenuDialog(_))
        }),
    ];
    for (command, is_open) in cases {
        let mut app = App::new().expect("test app");
        apply_action(&mut terminal(), &mut app, Action::Menu(command));
        assert!(is_open(&app.overlay), "{command:?}");
    }
}

/// Enter on a keyless row: the menu closes, and the command runs through the
/// same dispatcher a key would have reached.
#[test]
fn enter_on_a_keyless_row_runs_its_command_once_the_menu_has_closed() {
    let mut app = App::new().expect("test app");
    let mut terminal = terminal();
    app.set_menu(Menu::open(Heading::Options));
    dispatch_key(&mut terminal, &mut app, key_event(KeyCode::Enter));
    assert!(
        matches!(&app.overlay, Overlay::MenuDialog(_)),
        "the first Options row opened its page"
    );
    assert_eq!(app.pending_key, None, "and nothing is left waiting");
}

#[test]
fn enter_on_a_keyed_row_presses_its_key() {
    let mut app = App::new().expect("test app");
    let mut terminal = terminal();
    let mut menu = Menu::open(Heading::Commands);
    menu.row = menu.row_for_hotkey('f').expect("Find file");
    app.set_menu(menu);
    dispatch_key(&mut terminal, &mut app, key_event(KeyCode::Enter));
    assert!(
        matches!(app.overlay, Overlay::FindFile(_)),
        "Enter on Find file opens the Find file dialog"
    );
}

#[test]
fn f2_opens_the_user_menu() {
    let mut app = App::new().expect("test app");
    apply_action(&mut terminal(), &mut app, Action::UserMenu);
    // Far's F2 offers the user's own commands to choose from; editing that
    // file is a separate row in the Commands menu.
    assert!(matches!(app.overlay, Overlay::MenuDialog(_)));
}

#[test]
fn the_console_still_runs_a_menu_command() {
    let mut app = App::new().expect("test app");
    app.view = ViewMode::Console;
    apply_action(
        &mut terminal(),
        &mut app,
        Action::MenuCommand(MenuCommand::FindFile),
    );
    assert!(matches!(app.overlay, Overlay::FindFile(_)));
}

#[test]
fn enter_runs_an_associated_command() {
    let dir = tempfile::TempDir::new().expect("dir");
    std::fs::create_dir_all(dir.path().join(".ffm")).expect("ffm");
    std::fs::write(dir.path().join("note.txt"), "x").expect("file");
    std::fs::write(dir.path().join(".ffm/associations.txt"), "txt\t/bin/true\n").expect("assoc");
    let mut app = App::new().expect("app");
    app.left_panel = models::Panel::new(dir.path().to_path_buf()).expect("panel");
    app.right_panel = models::Panel::new(dir.path().to_path_buf()).expect("panel");
    assert!(app.get_active_panel().select_named("note.txt"));
    apply_action(&mut terminal(), &mut app, Action::Open);
}

#[test]
fn a_broken_associations_file_is_reported_instead_of_opening() {
    let dir = tempfile::TempDir::new().expect("dir");
    std::fs::create_dir_all(dir.path().join(".ffm/associations.txt")).expect("not a file");
    std::fs::write(dir.path().join("note.txt"), "x").expect("file");
    let mut app = App::new().expect("app");
    app.left_panel = models::Panel::new(dir.path().to_path_buf()).expect("panel");
    app.right_panel = models::Panel::new(dir.path().to_path_buf()).expect("panel");
    assert!(app.get_active_panel().select_named("note.txt"));
    apply_action(&mut terminal(), &mut app, Action::Open);
    assert!(matches!(app.status, status::Status::Error(_)));
}
