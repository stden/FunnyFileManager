//! Purpose: A shell that will not start, panel order and restored marks.

use super::*;

/// A shell that will not start still leaves its echoed line in the
/// transcript, followed by the reason, and counts as a failure.
#[test]
fn a_shell_that_will_not_start_is_recorded_as_an_error() {
    let dir = TempDir::new().expect("tempdir");
    let mut app = app_at(&dir);
    // The entries were read from a real directory; the run then happens in
    // one that is gone, which no shell can be started in.
    let gone = dir.path().join("gone");
    app.get_active_panel().path = gone.clone();
    run(&mut app, "echo hi");

    let lines: Vec<_> = app.console.visible(10).into_iter().cloned().collect();
    assert!(
        matches!(&lines[0], ConsoleLine::Command(echo) if echo.ends_with("> echo hi")),
        "{lines:?}"
    );
    // The folder is what is missing, not `echo`: the reason says so.
    let reason = format!(
        "cannot run 'echo hi': folder {} no longer exists",
        gone.display()
    );
    assert!(
        matches!(&lines[1], ConsoleLine::Error(why) if *why == reason),
        "{lines:?}"
    );
    assert_eq!(status_text(&app), "Apply command: 1 of 1 failed");
}

#[test]
fn a_failed_command_keeps_its_entry_marked() {
    let dir = TempDir::new().expect("tempdir");
    let mut app = app_at(&dir);
    app.get_active_panel()
        .marked
        .insert(crate::models::FileName::new("a.txt"));
    run(&mut app, "exit 1");
    assert_eq!(status_text(&app), "Apply command: 1 of 1 failed");
    assert!(
        app.get_active_panel().marked.contains("a.txt"),
        "failed command must not unmark the entry"
    );
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
    assert_eq!(panel.marked.len(), 0);
    panel.restore_marks();
    assert_eq!(panel.marked, original);
}

#[test]
fn empty_commands_succeed_preserve_name_noop_passes_and_cancellation_stops_remaining_batch() {
    let dir = TempDir::new().expect("tempdir");
    let mut app = app_at(&dir);
    let mut ctx = context::capture(&app, "a.txt");
    ctx.left.entry = crate::models::apply::Names::same("");
    assert!(super::super::run_entry(&mut app, "!+!", &ctx, dir.path(), dir.path()).unwrap());

    let names = vec!["a.txt".to_owned(), "b.txt".to_owned()];
    let failed = super::super::run_batch_with(
        &mut app,
        "echo !.",
        &names,
        dir.path(),
        dir.path(),
        |_, _| Err(super::super::ShellError::Cancelled),
    )
    .expect("batch");
    assert_eq!(failed, 2);
}

/// A cancel stops the batch at the entry it was pressed on: the shell is not
/// asked again, every entry counts as failed, and all of them stay marked.
#[test]
fn cancelling_the_shell_keeps_the_remaining_batch_unstarted() {
    let dir = TempDir::new().expect("tempdir");
    let mut app = app_at(&dir);
    fs::write(dir.path().join("b.txt"), "beta").expect("b");
    let names = ["a.txt", "b.txt"].map(str::to_owned);
    let panel = app.get_active_panel();
    panel.load_entries().expect("reload");
    for name in &names {
        panel
            .marked
            .insert(crate::models::FileName::new(name.as_str()));
    }
    let mut started = 0;
    let failed = super::super::run_batch_with(
        &mut app,
        "echo !.",
        &names,
        dir.path(),
        dir.path(),
        |_, _| {
            started += 1;
            Err(super::super::ShellError::Cancelled)
        },
    )
    .expect("batch");
    assert_eq!((failed, started), (2, 1));
    assert_eq!(app.get_active_panel().marked.len(), 2);
}
