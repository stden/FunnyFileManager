//! Unit tests: the Apply command dialog over a real directory — the field, the
//! buttons, and Enter running the command once per selected entry with every
//! run echoed into the console transcript.

use std::fs;

use crossterm::event::KeyCode;
use tempfile::TempDir;

use crate::console::ConsoleLine;
use crate::models::{App, FileName, Overlay, Panel};
use crate::popup::process_popup_input;
use crate::popup::tests::key;
use crate::status::Status;

fn app_at(dir: &TempDir) -> App {
    fs::write(dir.path().join("a.txt"), "alpha\n").expect("a.txt");
    fs::write(dir.path().join("b.txt"), "beta\n").expect("b.txt");
    let mut app = App::new().expect("test app");
    app.left_panel = Panel::new(dir.path().to_path_buf()).expect("left");
    app.right_panel = Panel::new(dir.path().to_path_buf()).expect("right");
    app
}

fn mark(app: &mut App, names: &[&str]) {
    for name in names {
        app.get_active_panel().marked.insert(FileName::new(*name));
    }
}

fn type_command(app: &mut App, text: &str) {
    crate::ops::apply::open(app);
    for ch in text.chars() {
        process_popup_input(app, key(KeyCode::Char(ch)));
    }
}

fn typed(app: &App) -> String {
    let Overlay::ApplyCommand(prompt) = &app.overlay else {
        panic!("the dialog is open");
    };
    prompt.command().to_owned()
}

fn console_lines(app: &App) -> Vec<ConsoleLine> {
    app.console.visible(100).into_iter().cloned().collect()
}

/// The `cwd> command` lines, in the order they were run.
fn echoed_commands(app: &App) -> Vec<String> {
    console_lines(app)
        .iter()
        .filter_map(|line| match line {
            ConsoleLine::Command(text) => Some(text.clone()),
            _ => None,
        })
        .collect()
}

fn status_text(app: &App) -> String {
    match &app.status {
        Status::CommandOutput(text) => text.clone(),
        other => panic!("a command summary, got {other:?}"),
    }
}

/// Opening sets the variant; Esc takes it down and runs nothing else.
#[test]
fn open_sets_the_variant_and_esc_closes_it() {
    let mut app = App::new().expect("test setup");
    crate::ops::apply::open(&mut app);
    assert!(matches!(app.overlay, Overlay::ApplyCommand(_)));

    process_popup_input(&mut app, key(KeyCode::Char('x')));
    assert!(
        matches!(app.overlay, Overlay::ApplyCommand(_)),
        "other keys are consumed"
    );

    process_popup_input(&mut app, key(KeyCode::Esc));
    assert!(!app.overlay.is_active());
    assert!(app.console.is_empty(), "nothing ran");
}

#[test]
fn typing_edits_the_field_and_tab_moves_between_the_buttons() {
    let dir = TempDir::new().expect("tempdir");
    let mut app = app_at(&dir);
    type_command(&mut app, "lsx");
    process_popup_input(&mut app, key(KeyCode::Backspace));
    assert_eq!(typed(&app), "ls");

    process_popup_input(&mut app, key(KeyCode::Tab));
    let Overlay::ApplyCommand(prompt) = &app.overlay else {
        panic!("still open");
    };
    assert!(prompt.on_cancel());
    process_popup_input(&mut app, key(KeyCode::Tab));
    let Overlay::ApplyCommand(prompt) = &app.overlay else {
        panic!("still open");
    };
    assert!(!prompt.on_cancel(), "Tab goes back to OK");
}

/// The field is a real line editor: Left, Home, End and Delete move and cut
/// mid-line, and Ctrl / Alt chords are not typed into it.
#[test]
fn the_field_edits_mid_line_and_ignores_chords() {
    use crossterm::event::{KeyEvent, KeyModifiers};

    let dir = TempDir::new().expect("tempdir");
    let mut app = app_at(&dir);
    type_command(&mut app, "cp !.! !.bak");
    for _ in 0..4 {
        process_popup_input(&mut app, key(KeyCode::Left));
    }
    process_popup_input(&mut app, key(KeyCode::Char('.')));
    assert_eq!(typed(&app), "cp !.! !..bak");
    process_popup_input(&mut app, key(KeyCode::Delete));
    assert_eq!(typed(&app), "cp !.! !.bak");
    process_popup_input(&mut app, key(KeyCode::Home));
    process_popup_input(&mut app, key(KeyCode::Right));
    process_popup_input(&mut app, key(KeyCode::Right));
    process_popup_input(&mut app, key(KeyCode::Char('x')));
    assert_eq!(typed(&app), "cpx !.! !.bak");
    process_popup_input(&mut app, key(KeyCode::End));
    process_popup_input(&mut app, key(KeyCode::Backspace));
    assert_eq!(typed(&app), "cpx !.! !.ba");

    for modifiers in [KeyModifiers::CONTROL, KeyModifiers::ALT] {
        process_popup_input(&mut app, KeyEvent::new(KeyCode::Char('q'), modifiers));
    }
    assert_eq!(typed(&app), "cpx !.! !.ba", "chords are not text");
    let Overlay::ApplyCommand(prompt) = &app.overlay else {
        panic!("still open");
    };
    assert!(!prompt.on_cancel(), "and arrows do not move the buttons");
}

/// F10 closes like Esc; with Escape turned off in the dialog settings only
/// F10 does, as in every other prompt.
#[test]
fn f10_closes_and_esc_honours_the_dialog_setting() {
    let dir = TempDir::new().expect("tempdir");
    let mut app = app_at(&dir);
    type_command(&mut app, "ls");
    process_popup_input(&mut app, key(KeyCode::F(10)));
    assert!(!app.overlay.is_active());

    app.settings
        .set(crate::menu::SettingsPage::Dialog, "off")
        .expect("dialog setting");
    type_command(&mut app, "ls");
    process_popup_input(&mut app, key(KeyCode::Esc));
    assert!(app.overlay.is_active(), "Esc is swallowed");
    process_popup_input(&mut app, key(KeyCode::F(10)));
    assert!(!app.overlay.is_active());
    assert!(app.console.is_empty(), "nothing ran");
}

/// Enter on Cancel — and Enter on an empty field — close without running.
#[test]
fn enter_on_cancel_or_on_nothing_runs_nothing() {
    let dir = TempDir::new().expect("tempdir");
    let mut app = app_at(&dir);
    type_command(&mut app, "touch made-by-apply");
    process_popup_input(&mut app, key(KeyCode::Tab));
    process_popup_input(&mut app, key(KeyCode::Enter));
    assert!(!app.overlay.is_active());
    assert!(
        !dir.path().join("made-by-apply").exists(),
        "cancel ran nothing"
    );

    type_command(&mut app, "   ");
    process_popup_input(&mut app, key(KeyCode::Enter));
    assert!(!app.overlay.is_active());
    assert!(app.console.is_empty(), "a blank command is not echoed");
}

/// The Far idiom: `cp !.! !.bak` over two marked files makes two backups, each
/// run is echoed with its own substituted line, and the panel shows the new
/// files without a manual refresh.
#[test]
fn enter_runs_the_command_once_per_marked_entry() {
    let dir = TempDir::new().expect("tempdir");
    let mut app = app_at(&dir);
    mark(&mut app, &["a.txt", "b.txt"]);
    type_command(&mut app, "cp !.! !.bak");
    process_popup_input(&mut app, key(KeyCode::Enter));

    assert!(!app.overlay.is_active());
    assert_eq!(
        fs::read_to_string(dir.path().join("a.bak")).expect("a.bak"),
        "alpha\n"
    );
    assert_eq!(
        fs::read_to_string(dir.path().join("b.bak")).expect("b.bak"),
        "beta\n"
    );

    let cwd = dir.path().display().to_string();
    assert_eq!(
        echoed_commands(&app),
        vec![
            format!("{cwd}> cp a.txt a.bak"),
            format!("{cwd}> cp b.txt b.bak")
        ]
    );
    assert_eq!(status_text(&app), "Apply command: 2 run");
    assert!(
        app.get_active_panel()
            .entries
            .iter()
            .any(|entry| entry.name == "a.bak"),
        "the panel was re-read"
    );
}

/// Without marks the entry under the cursor is the one, and what it printed
/// lands in the transcript as output.
#[test]
fn without_marks_the_cursor_entry_is_the_one() {
    let dir = TempDir::new().expect("tempdir");
    let mut app = app_at(&dir);
    assert!(app.get_active_panel().select_named("b.txt"));
    type_command(&mut app, "echo !` !");
    process_popup_input(&mut app, key(KeyCode::Enter));

    let lines = console_lines(&app);
    assert!(
        lines.contains(&ConsoleLine::Output("txt b".to_owned())),
        "{lines:?}"
    );
    assert_eq!(status_text(&app), "Apply command: 1 run");
}

/// A failing run is counted, its stderr is kept, and the rest still run.
#[test]
fn failures_are_counted_and_their_stderr_kept() {
    let dir = TempDir::new().expect("tempdir");
    let mut app = app_at(&dir);
    mark(&mut app, &["a.txt", "b.txt"]);
    let command = if cfg!(windows) {
        "if !.!==b.txt (echo no-!.!>&2& exit /b 1)"
    } else {
        "test !.! = a.txt || { echo no-!.! >&2; false; }"
    };
    type_command(&mut app, command);
    process_popup_input(&mut app, key(KeyCode::Enter));

    let lines = console_lines(&app);
    assert!(
        lines.contains(&ConsoleLine::Error("no-b.txt".to_owned())),
        "{lines:?}"
    );
    assert_eq!(status_text(&app), "Apply command: 1 of 2 failed");
}

/// A new list contains the remaining selection for each run. Each file is
/// removed afterwards, including when the command fails.
#[test]
fn the_list_file_holds_every_selected_name_and_is_removed_after() {
    let dir = TempDir::new().expect("tempdir");
    let mut app = app_at(&dir);
    mark(&mut app, &["a.txt", "b.txt"]);
    type_command(&mut app, "cp \"!@U!\" list-of-!.!");
    process_popup_input(&mut app, key(KeyCode::Enter));

    let copied = fs::read_to_string(dir.path().join("list-of-a.txt")).expect("copied list");
    assert_eq!(copied.lines().collect::<Vec<_>>(), ["a.txt", "b.txt"]);
    let second = fs::read_to_string(dir.path().join("list-of-b.txt")).expect("second list");
    assert_eq!(second.lines().collect::<Vec<_>>(), ["b.txt"]);
    for command in echoed_commands(&app) {
        let list_path = command.split('"').nth(1).expect("quoted list path");
        assert!(
            !std::path::Path::new(list_path).exists(),
            "list removed: {list_path}"
        );
    }
}

/// A panel standing on `..` alone has nothing to apply to, and says so.
#[test]
fn nothing_selected_is_reported_not_run() {
    let dir = TempDir::new().expect("tempdir");
    let mut app = App::new().expect("test app");
    app.left_panel = Panel::new(dir.path().to_path_buf()).expect("left");
    type_command(&mut app, "touch made-by-apply");
    process_popup_input(&mut app, key(KeyCode::Enter));

    assert!(!dir.path().join("made-by-apply").exists());
    assert_eq!(status_text(&app), "Apply command: nothing selected");
}
