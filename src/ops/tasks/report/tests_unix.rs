//! Purpose: Tests for the text report on Unix, where a shell stands in for
//! `ps`: the viewer a report fills, and the error a failed report gives.

use std::process::Command;

use tempfile::TempDir;

use super::show;
use crate::models::{App, Overlay, Panel};

fn app(root: &TempDir) -> App {
    let panel = || Panel::from_entries(root.path().to_path_buf(), Vec::new());
    App::with_panels(panel(), panel())
}

fn shell(script: &str) -> Command {
    let mut command = Command::new("sh");
    command.args(["-c", script]);
    command
}

#[test]
fn a_report_shows_each_line_it_printed() {
    let root = TempDir::new().expect("dir");
    let mut app = app(&root);
    show(&mut app, &mut shell("printf 'PID\\n42\\n'")).expect("the report runs");
    let Overlay::Viewer(viewer) = &app.overlay else {
        panic!("the report is open");
    };
    assert_eq!(viewer.title(), "Task list");
    assert_eq!(viewer.lines, ["PID", "42"]);
}

#[test]
fn a_report_that_fails_is_its_error_output_and_opens_nothing() {
    let root = TempDir::new().expect("dir");
    let mut app = app(&root);
    let error = show(&mut app, &mut shell("echo no processes >&2; exit 3")).unwrap_err();
    assert_eq!(error.to_string(), "no processes\n");
    assert!(!app.overlay.is_active());
}
