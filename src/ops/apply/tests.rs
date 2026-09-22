//! Unit tests: the batch's summary line, and the two ways a run can fail
//! before the shell even gets the command — a `!@` list that cannot be
//! written, and a shell that will not start.

use std::fs;

use tempfile::TempDir;

use super::{run, run_with_list_dir, summary};
use crate::console::ConsoleLine;
use crate::models::{App, Panel};
use crate::status::Status;

fn app_at(dir: &TempDir) -> App {
    fs::write(dir.path().join("a.txt"), "alpha\n").expect("a.txt");
    let mut app = App::new().expect("test app");
    app.left_panel = Panel::new(dir.path().to_path_buf()).expect("left");
    assert!(app.get_active_panel().select_named("a.txt"));
    app
}

fn status_text(app: &App) -> String {
    match &app.status {
        Status::CommandOutput(text) => text.clone(),
        other => panic!("a command summary, got {other:?}"),
    }
}

#[test]
fn the_summary_counts_runs_and_failures() {
    assert_eq!(
        summary(3, 0),
        Status::CommandOutput("Apply command: 3 run".to_owned())
    );
    assert_eq!(
        summary(5, 2),
        Status::CommandOutput("Apply command: 2 of 5 failed".to_owned())
    );
}

/// A `!@` list that cannot be written stops the batch before its first run:
/// the shell would otherwise be handed a path to a file that does not exist.
#[test]
fn a_list_that_cannot_be_written_is_reported_and_nothing_runs() {
    let dir = TempDir::new().expect("tempdir");
    let mut app = app_at(&dir);
    run_with_list_dir(&mut app, "cp !@! copy", &dir.path().join("no-such-dir"));

    assert!(
        status_text(&app).starts_with("Apply command: cannot write the !@ list: "),
        "{}",
        status_text(&app)
    );
    assert!(app.console.is_empty(), "nothing was echoed or run");
    assert!(!dir.path().join("copy").exists());
}

#[test]
fn far_panel_prefixes_can_copy_from_the_passive_panel() {
    let active = TempDir::new().expect("active");
    let passive = TempDir::new().expect("passive");
    let mut app = app_at(&active);
    fs::write(passive.path().join("other file.txt"), "passive contents").expect("other");
    app.right_panel = Panel::new(passive.path().to_owned()).expect("passive panel");
    assert!(app.right_panel.select_named("other file.txt"));
    run(&mut app, "cp \"!#!\\!.!\" \"!^!.bak\"");
    assert_eq!(
        status_text(&app),
        "Apply command: 1 run",
        "{:?}",
        app.console.visible(10)
    );
    assert_eq!(
        fs::read_to_string(active.path().join("a.bak")).expect("copy"),
        "passive contents"
    );
}

#[test]
fn far_list_modifiers_write_full_quoted_utf8_paths() {
    let dir = TempDir::new().expect("tempdir");
    let mut app = app_at(&dir);
    run(&mut app, "cp \"!@UFQS!\" listing.txt");
    assert_eq!(
        status_text(&app),
        "Apply command: 1 run",
        "{:?}",
        app.console.visible(10)
    );
    let expected = format!(
        "\"{}\"{}",
        dir.path()
            .join("a.txt")
            .display()
            .to_string()
            .replace('\\', "/"),
        line_ending()
    );
    assert_eq!(
        fs::read_to_string(dir.path().join("listing.txt")).expect("list"),
        expected
    );
}

#[test]
fn far_utf16_lists_have_no_bom_and_are_cleaned_up() {
    let dir = TempDir::new().expect("tempdir");
    let lists = TempDir::new().expect("lists");
    let mut app = app_at(&dir);
    run_with_list_dir(&mut app, "cp \"!@W!\" listing.txt", lists.path());
    assert_eq!(
        status_text(&app),
        "Apply command: 1 run",
        "{:?}",
        app.console.visible(10)
    );
    let expected: Vec<_> = format!("a.txt{}", line_ending())
        .encode_utf16()
        .flat_map(u16::to_le_bytes)
        .collect();
    assert_eq!(
        fs::read(dir.path().join("listing.txt")).expect("list"),
        expected
    );
    assert_eq!(fs::read_dir(lists.path()).expect("lists").count(), 0);
}

fn line_ending() -> &'static str {
    if cfg!(windows) {
        "\r\n"
    } else {
        "\n"
    }
}

/// A shell that will not start still leaves its echoed line in the
/// transcript, followed by the reason, and counts as a failure.
#[test]
fn a_shell_that_will_not_start_is_recorded_as_an_error() {
    let dir = TempDir::new().expect("tempdir");
    let mut app = app_at(&dir);
    // The entries were read from a real directory; the run then happens in
    // one that is gone, which no shell can be started in.
    app.get_active_panel().path = dir.path().join("gone");
    run(&mut app, "echo hi");

    let lines: Vec<_> = app.console.visible(10).into_iter().cloned().collect();
    assert!(
        matches!(&lines[0], ConsoleLine::Command(echo) if echo.ends_with("> echo hi")),
        "{lines:?}"
    );
    assert!(
        matches!(&lines[1], ConsoleLine::Error(why) if why.starts_with("cannot run 'echo hi': ")),
        "{lines:?}"
    );
    assert_eq!(status_text(&app), "Apply command: 1 of 1 failed");
}

#[test]
fn commands_follow_panel_order_and_processed_marks_can_be_restored() {
    let dir = TempDir::new().expect("tempdir");
    let mut app = app_at(&dir);
    fs::write(dir.path().join("b.txt"), "beta").expect("b");
    let panel = app.get_active_panel();
    panel.load_entries().expect("reload");
    panel.entries.reverse();
    panel.marked.extend([
        crate::models::FileName::new("a.txt"),
        crate::models::FileName::new("b.txt"),
    ]);
    let original = panel.marked.clone();
    run(&mut app, "echo !.! !&q");
    let output: Vec<_> = app
        .console
        .visible(20)
        .into_iter()
        .filter_map(|line| {
            if let ConsoleLine::Output(text) = line {
                Some(text.as_str())
            } else {
                None
            }
        })
        .collect();
    assert_eq!(output, ["b.txt b.txt a.txt", "a.txt a.txt"]);
    let panel = app.get_active_panel();
    assert!(panel.marked.is_empty());
    panel.restore_marks();
    assert_eq!(panel.marked, original);
}

#[test]
fn literal_list_symbols_do_not_create_files() {
    let dir = TempDir::new().expect("tempdir");
    let mut app = app_at(&dir);
    run_with_list_dir(&mut app, "echo !!@! {}", &dir.path().join("absent"));
    assert_eq!(status_text(&app), "Apply command: 1 run");
    assert!(app
        .console
        .visible(10)
        .iter()
        .any(|line| matches!(line, ConsoleLine::Output(text) if text == "!@a {}")));
}

#[test]
fn native_short_names_resolve_to_the_same_file() {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join("long filename with spaces.txt");
    fs::write(&path, "contents").expect("file");
    let short = super::context::short_path(&path);
    assert_eq!(fs::read_to_string(short).expect("short path"), "contents");
    let missing = dir.path().join("nonexistent file.txt");
    assert_eq!(super::context::short_path(&missing), missing);
}

#[test]
fn preserve_short_name_restores_a_recreated_long_name() {
    let dir = TempDir::new().expect("tempdir");
    let names = crate::models::apply::Names {
        long: "original long name.txt".into(),
        short: "ORIG~1.TXT".into(),
    };
    fs::write(dir.path().join(&names.short), "recreated").expect("short file");
    super::context::restore_long_name(dir.path(), &names).expect("restore");
    assert_eq!(
        fs::read_to_string(dir.path().join(&names.long)).expect("long name"),
        "recreated"
    );
    assert!(!dir.path().join(&names.short).exists());
}

#[test]
fn preserve_short_name_never_replaces_an_existing_long_name() {
    let dir = TempDir::new().expect("tempdir");
    let names = crate::models::apply::Names {
        long: "existing.txt".into(),
        short: "OTHER.TXT".into(),
    };
    fs::write(dir.path().join(&names.short), "other").expect("short file");
    fs::write(dir.path().join(&names.long), "original").expect("long file");
    super::context::restore_long_name(dir.path(), &names).expect("preserve");
    assert_eq!(
        fs::read_to_string(dir.path().join(&names.long)).expect("long name"),
        "original"
    );
    assert_eq!(
        fs::read_to_string(dir.path().join(&names.short)).expect("short name"),
        "other"
    );
}
