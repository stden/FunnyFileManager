//! Purpose: A start without a terminal is reported by the real binary on
//! stderr, as a diagnosis rather than a panic.

#![cfg(unix)]

use std::{
    io,
    process::{Command, Output, Stdio},
};

#[test]
fn startup_without_a_terminal_reports_failure_on_stderr() -> io::Result<()> {
    const CHILD: &str = "FFM_TEST_NO_TERMINAL";
    if std::env::var_os(CHILD).is_none() {
        let output = Command::new(std::env::current_exe()?)
            .args([
                "--exact",
                "startup_without_a_terminal_reports_failure_on_stderr",
            ])
            .env(CHILD, "1")
            .output()?;
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        return Ok(());
    }
    let output = start_without_terminal()?;
    assert_eq!(output.status.code(), Some(1));
    let diagnostic = String::from_utf8_lossy(&output.stderr);
    assert!(diagnostic.starts_with("ffm: "), "{diagnostic}");
    assert!(!diagnostic.contains("panicked"), "{diagnostic}");
    Ok(())
}

/// Run ffm from a session of its own, so it has no controlling terminal, with
/// no `tput` on `PATH` and no size in the environment to fall back on.
fn start_without_terminal() -> io::Result<Output> {
    rustix::process::setsid()?;
    Command::new(env!("CARGO_BIN_EXE_ffm"))
        .env("PATH", "")
        .env("FFM_LOG", "0")
        .env_remove("COLUMNS")
        .env_remove("LINES")
        .stdin(Stdio::null())
        .output()
}
