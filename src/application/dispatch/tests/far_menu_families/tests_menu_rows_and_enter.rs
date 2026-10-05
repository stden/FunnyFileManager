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
    let config = tempfile::TempDir::new().expect("config dir");
    for (command, is_open) in cases {
        let mut app = app_on(config.path(), Some(config.path()));
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
    let ran = dir.path().join("ran");
    std::fs::write(dir.path().join("note.txt"), "x").expect("file");
    let association = format!("txt\ttouch {}\n", ran.display());
    std::fs::write(dir.path().join("associations.txt"), association).expect("assoc");
    let mut app = app_on(dir.path(), Some(dir.path()));
    assert!(app.get_active_panel().select_named("note.txt"));
    apply_action(&mut terminal(), &mut app, Action::Open);
    assert!(ran.exists(), "{:?}", app.status);
}

#[test]
fn a_broken_associations_file_is_reported_instead_of_opening() {
    let dir = tempfile::TempDir::new().expect("dir");
    std::fs::create_dir_all(dir.path().join("associations.txt")).expect("not a file");
    std::fs::write(dir.path().join("note.txt"), "x").expect("file");
    let mut app = app_on(dir.path(), Some(dir.path()));
    assert!(app.get_active_panel().select_named("note.txt"));
    apply_action(&mut terminal(), &mut app, Action::Open);
    assert!(matches!(app.status, status::Status::Error(_)));
}

/// With no configuration directory — HOME unset, as under `env -i`, cron or
/// a service — the associations and the user menu were read from `.ffm/` in
/// the active panel's folder, so Enter on a file in a cloned repository or an
/// unpacked archive ran a command its author wrote. A folder's own files are
/// never configuration: without a configuration directory there is none.
#[test]
fn a_folders_own_ffm_files_are_never_read_or_run() {
    let dir = tempfile::TempDir::new().expect("dir");
    let pwned = dir.path().join("pwned");
    std::fs::create_dir_all(dir.path().join(".ffm")).expect("ffm");
    std::fs::write(dir.path().join("README.txt"), "hello").expect("file");
    let association = format!("txt\ttouch {}\n", pwned.display());
    std::fs::write(dir.path().join(".ffm/associations.txt"), association).expect("assoc");
    std::fs::write(dir.path().join(".ffm/user-menu.txt"), "Build\tmake\n").expect("menu");
    let mut app = app_on(dir.path(), None);
    assert!(app.get_active_panel().select_named("README.txt"));
    apply_action(&mut terminal(), &mut app, Action::Open);
    assert!(!pwned.exists(), "the folder's association ran");
    apply_action(&mut terminal(), &mut app, Action::UserMenu);
    let Overlay::MenuDialog(dialog) = &app.overlay else {
        panic!("the user menu is open: {:?}", app.status);
    };
    let models::MenuDialog::Choices { rows, .. } = dialog.as_ref() else {
        panic!("choices");
    };
    assert!(rows.iter().all(|row| row.label != "Build"), "{rows:?}");
}

/// An app with both panels on `dir`, keeping its settings — and so the user
/// menu and the associations — in `config` when there is one.
fn app_on(dir: &Path, config: Option<&Path>) -> App {
    let panel = models::Panel::new(dir.to_path_buf()).expect("panel");
    let mut app = App::from_panels(panel.clone(), panel);
    if let Some(config) = config {
        app.settings = models::settings::Settings::with_file(config.join("settings.yaml"));
    }
    app
}
