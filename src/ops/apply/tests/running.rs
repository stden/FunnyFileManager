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
