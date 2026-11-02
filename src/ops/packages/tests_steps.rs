//! Purpose: Direct unit tests for `run_step` process spawning and stderr capture.

use super::run_step;
use std::process::Command;

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
}
