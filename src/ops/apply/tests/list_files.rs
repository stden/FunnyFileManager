//! Purpose: The summary, and the list files `!@!` and its kin write.

use super::*;

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
    assert_eq!(app.console.len(), 0, "nothing was echoed or run");
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
