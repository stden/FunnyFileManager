use super::*;

/// `run_with` only wraps the runner in log lines; a successful run must stay
/// successful and the runner must be entered exactly once.
#[test]
fn run_with_runs_the_runner_once_and_keeps_its_success() {
    let mut calls = 0_u32;

    run_with(|| {
        calls += 1;
        Ok(())
    })
    .expect("a successful runner must not be turned into an error");

    assert_eq!(calls, 1, "the runner must be invoked exactly once");
}

/// The wrapper logs the failure but must not swallow or rewrite it: `main`
/// prints this error, so its text is what the user sees.
#[test]
fn run_with_propagates_the_runner_error_verbatim() {
    let error = run_with(|| Err(io::Error::other("main exploded").into()))
        .expect_err("a failing runner must not be reported as success");

    assert_eq!(error.to_string(), "main exploded");
}

/// Logging is opt-in. With `FFM_LOG` unset nothing may reach disk — that gate
/// is the reason a password typed into the editor never lands in a log file.
#[test]
fn log_error_writes_nothing_while_logging_is_off() {
    // `log_dir` memoizes its answer in a `OnceLock`, so `FFM_LOG` can only be
    // observed in the state the whole test binary shares. Take the env lock so
    // the check cannot straddle another test's env swap.
    let _lock = crate::test_support::editor_env_lock();
    if std::env::var_os("FFM_LOG").is_some() {
        return; // this run deliberately opted into logging
    }

    let log = std::env::temp_dir().join("ffm_errors.log");
    let before = fs::metadata(&log).map_or(0, |meta| meta.len());

    log_error("token=hunter2");

    let after = fs::metadata(&log).map_or(0, |meta| meta.len());
    assert_eq!(
        before,
        after,
        "an error line reached {} with FFM_LOG unset",
        log.display()
    );
}

/// Name of the child test below, as libtest spells it on the command line.
const LOG_CHILD_TEST: &str =
    "diagnostics::tests::lifecycle::log_lines_reach_disk_when_ffm_log_is_on";

/// The opt-in half of the logging gate, which decides *where* an enabled log
/// lands and under what name.
///
/// `log_dir` memoises its answer in a process-wide `OnceLock` and every test
/// that drives the event loop reads it first, so `FFM_LOG=1` can only be
/// observed in a fresh process. Re-run this binary for the single ignored test
/// below with the temp directory pointed at one of our own, then read back
/// exactly what it wrote.
#[test]
fn ffm_log_opt_in_writes_both_logs_into_the_temp_directory() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let child = run_log_child(temp_dir.path())?;
    assert!(
        child.status.success(),
        "the child run failed:\n{}\n{}",
        String::from_utf8_lossy(&child.stdout),
        String::from_utf8_lossy(&child.stderr)
    );

    let actions = temp_dir.path().join("ffm_actions.log");
    let errors = temp_dir.path().join("ffm_errors.log");
    assert!(
        fs::read_to_string(&actions)?.contains("] opened the panel"),
        "the action never reached {}",
        actions.display()
    );
    // The two logs are separate files: an error in the action log (or the other
    // way round) means the pair was crossed.
    let error_lines = fs::read_to_string(&errors)?;
    assert!(
        error_lines.contains("] ERROR: disk on fire"),
        "the error never reached {}: {error_lines}",
        errors.display()
    );

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for log in [&actions, &errors] {
            let mode = fs::metadata(log)?.permissions().mode() & 0o777;
            assert_eq!(mode, 0o600, "{} is readable by other users", log.display());
        }
    }
    Ok(())
}

/// Child of [`ffm_log_opt_in_writes_both_logs_into_the_temp_directory`]: it
/// only writes the two lines that test reads back. Ignored, because it is
/// meaningless in a process that did not opt in.
#[test]
#[ignore = "re-run with FFM_LOG=1 by ffm_log_opt_in_writes_both_logs_into_the_temp_directory"]
fn log_lines_reach_disk_when_ffm_log_is_on() {
    let expected = std::env::temp_dir();
    assert_eq!(
        log_dir(),
        Some(expected.as_path()),
        "FFM_LOG=1 did not reach this process"
    );

    log_action("opened the panel");
    log_error("disk on fire");
}

fn run_log_child(directory: &std::path::Path) -> io::Result<std::process::Output> {
    std::process::Command::new(std::env::current_exe()?)
        .args(["--ignored", "--exact", LOG_CHILD_TEST])
        .env("FFM_LOG", "1")
        .env("TMPDIR", directory)
        .env("TMP", directory)
        .env("TEMP", directory)
        .output()
}
