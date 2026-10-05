//! Purpose: Pin the Language servers dialog's first and last rows: the
//! settings file F2 writes or why the dialog is read only, a deep path that
//! keeps its file name, and a notice that lasts one key.

use super::*;

/// A read-only dialog names the file and the reason, and its key bar drops
/// the keys that would change the table.
#[test]
fn a_read_only_dialog_says_why_on_its_first_row() {
    let machine = LspMachine::new();
    machine.write_settings("{broken");
    let mut app = machine.opened();
    let buffer = draw(&mut app, 130, 24);
    let screen = buffer_string(&buffer);
    assert!(
        screen.contains("— read only: settings.yaml does not parse: "),
        "{screen}"
    );
    assert_eq!(fg(&buffer, "— read only"), palette::STATUS_FAILED);
    assert!(!screen.contains("2Save"), "{screen}");
    assert!(screen.contains("F3 View  F5 Check  Esc Close"), "{screen}");
    assert!(
        screen.contains(
            "Read only: settings.yaml does not parse: did not find expected ',' or '}' at \
             line 2 column 1, while parsing a flow mapping."
        ),
        "the message row has room for the whole reason\n{screen}"
    );
}

/// A directory name long enough that no 80-column row holds a path through
/// it whole, whatever the temp directory's own length.
const DEEP: &str = "a-settings-directory-long-enough-that-no-row-holds-the-whole-path";

/// A settings file in a directory deeper than the row is wide.
fn deep_settings(machine: &LspMachine) -> PathBuf {
    let directory = machine.home.path().join(DEEP);
    std::fs::create_dir(&directory).expect("directory");
    directory.join(FILE_NAME)
}

/// The dialog open on the settings file `file`.
fn opened_on(machine: &LspMachine, file: &Path) -> App {
    let mut app = machine.app();
    app.settings = Settings::with_file(file.to_owned());
    crate::ops::lsp_editor::open(&mut app);
    app
}

/// The Settings row keeps the file's name however deep it sits: the path
/// gives up its head. On the Windows runner the temp directory alone was
/// longer than the row, and the name was what fell off its end.
#[test]
fn a_deep_settings_file_keeps_its_name() {
    let machine = LspMachine::new();
    let file = deep_settings(&machine);
    let mut app = opened_on(&machine, &file);
    let screen = buffer_string(&draw(&mut app, 80, 24));
    let row = framed_row(&screen, "Settings: ");
    assert!(row.starts_with("Settings: …"), "{row:?}");
    assert!(quotes_path(&row, &file), "{row:?}");
}

/// A read-only file deep in the tree keeps its name and the start of the
/// reason: the path is cut before the reason is.
#[test]
fn a_deep_read_only_file_keeps_its_name_and_the_reason() {
    let machine = LspMachine::new();
    let file = deep_settings(&machine);
    std::fs::write(&file, "{broken").expect("settings");
    let mut app = opened_on(&machine, &file);
    let screen = buffer_string(&draw(&mut app, 80, 24));
    let row = framed_row(&screen, "Settings: ");
    let expected = format!(
        "Settings: …{} — read only: {FILE_NAME} does not parse",
        last_component(FILE_NAME)
    );
    assert!(row.starts_with(&expected), "{row:?}");
}

/// A session without a settings file says so where the path would be.
#[test]
fn a_session_without_a_settings_file_says_so_on_its_first_row() {
    let machine = LspMachine::new();
    let mut app = machine.app();
    app.settings.persistence = crate::history::Persistence::SessionOnly;
    crate::ops::lsp_editor::open(&mut app);
    let buffer = draw(&mut app, 80, 24);
    let screen = buffer_string(&buffer);
    assert_eq!(
        framed_row(&screen, "Settings: "),
        "Settings: none for this session — read only"
    );
    assert_eq!(fg(&buffer, "none for this session"), palette::STATUS_FAILED);
}

/// A notice lasts one key: the next edit shows what is unsaved again, so a
/// "Saved to …" never stands over changes made after it.
#[test]
fn a_notice_gives_way_to_the_changes_on_the_next_key() {
    let machine = LspMachine::new();
    let mut app = machine.opened();
    press(&mut app, KeyCode::F(5));
    assert!(buffer_string(&draw(&mut app, 100, 30)).contains("Checked every server again."));
    press(&mut app, KeyCode::Char(' '));
    let screen = buffer_string(&draw(&mut app, 100, 30));
    assert!(
        screen.contains("Modified: bat. F2 saves only what changed."),
        "{screen}"
    );
    press(&mut app, KeyCode::F(2));
    assert!(buffer_string(&draw(&mut app, 100, 30)).contains("Saved to "));
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Char(' '));
    let screen = buffer_string(&draw(&mut app, 100, 30));
    assert!(!screen.contains("Saved to "), "{screen}");
    assert!(
        screen.contains("Modified: css. F2 saves only what changed."),
        "{screen}"
    );
}
