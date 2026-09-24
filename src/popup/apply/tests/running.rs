//! Purpose: running the Apply command — once per marked entry or the cursor's, failures counted with their stderr, the list file removed after, and nothing selected reported.

use super::*;

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
