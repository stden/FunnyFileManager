//! Purpose: Tests for the text report the task list falls back to: the
//! command each platform runs.

use std::process::Command;

use tempfile::TempDir;

use super::{report_command, show};
use crate::models::{App, Panel};

#[cfg(not(windows))]
#[test]
fn unix_reports_with_ps_and_its_four_columns() {
    let command = report_command();
    assert_eq!(command.get_program(), "ps");
    let args: Vec<_> = command.get_args().collect();
    assert_eq!(args, ["-eo", "pid,comm,%cpu,%mem"]);
}

#[cfg(windows)]
#[test]
fn windows_reports_with_tasklist() {
    let command = report_command();
    assert_eq!(command.get_program(), "tasklist");
    assert_eq!(command.get_args().count(), 0);
}

#[test]
fn a_report_that_fails_returns_stderr_and_keeps_overlay_closed() {
    let root = TempDir::new().expect("dir");
    let panel = || Panel::from_entries(root.path().to_path_buf(), Vec::new());
    let mut app = App::with_panels(panel(), panel());
    #[cfg(windows)]
    let mut cmd = {
        let mut c = Command::new("cmd.exe");
        c.args(["/D", "/C", "echo no processes 1>&2 & exit /b 3"]);
        c
    };
    #[cfg(not(windows))]
    let mut cmd = {
        let mut c = Command::new("sh");
        c.args(["-c", "echo no processes >&2; exit 3"]);
        c
    };
    let error = show(&mut app, &mut cmd).expect_err("non-zero status");
    assert!(error.to_string().contains("no processes"));
    assert!(!app.overlay.is_active());
}
