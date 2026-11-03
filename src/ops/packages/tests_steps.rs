//! Purpose: Direct unit tests for `run_step`, `execute`, and `outcome_status`.

use super::{execute, outcome_status, run_step};
use crate::{
    models::{App, Panel},
    packages::{Elevation, Manager, UpdatePlan},
    status::Status,
};
use std::{io, process::Command};

#[test]
fn run_step_captures_stderr_and_reports_spawn_failure() {
    #[cfg(windows)]
    let cmd = {
        let mut c = Command::new("cmd.exe");
        c.args(["/D", "/C", "echo step-err 1>&2 & exit /b 9"]);
        c
    };
    #[cfg(not(windows))]
    let cmd = {
        let mut c = Command::new("sh");
        c.args(["-c", "echo step-err >&2; exit 9"]);
        c
    };
    let (status, stderr) = run_step(cmd).expect("spawned");
    assert_eq!(status.code(), Some(9));
    assert!(stderr.contains("step-err"), "{stderr}");

    let missing = Command::new("definitely-missing-ffm-binary-xyz");
    let err = run_step(missing).expect_err("missing binary");
    assert!(err.to_string().contains("cannot start"));
    assert_eq!(
        outcome_status(Err(io::Error::other("aborted")), true),
        Status::PackageUpdateCancelled
    );
}

#[cfg(windows)]
#[test]
fn execute_runs_current_user_fixture_script_on_windows() {
    let temp = tempfile::TempDir::new().expect("tempdir");
    let script = temp.path().join("fake-mgr.cmd");
    std::fs::write(&script, "@echo off\r\nexit /b 0\r\n").expect("write cmd");
    let mut app = App::with_panels(
        Panel::new(temp.path().to_owned()).expect("left"),
        Panel::new(temp.path().to_owned()).expect("right"),
    );
    let plan = UpdatePlan {
        manager: Manager::Brew,
        executable: script,
        elevation: Elevation::CurrentUser,
    };
    let summary = execute(&plan, &mut app).expect("execute cmd");
    assert!(summary.contains("completed"), "{summary}");
}
