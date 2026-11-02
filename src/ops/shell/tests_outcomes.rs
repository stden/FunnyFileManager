//! Purpose: Pin how a shell run that produced no output ends: cancelled by
//! the user, stopped by the time limit.

use super::*;
use tempfile::TempDir;

/// Long enough that only a cancel or the time limit can end it.
#[cfg(windows)]
const LONG_COMMAND: &str = "ping -n 30 127.0.0.1 >nul";
#[cfg(not(windows))]
const LONG_COMMAND: &str = "sleep 30";

#[test]
fn a_cancelled_command_is_reported_as_cancelled_not_as_a_timeout() {
    let temp = TempDir::new().expect("tempdir");
    let started = Instant::now();
    let error = run_until(LONG_COMMAND, temp.path(), Duration::from_secs(10), || true)
        .expect_err("the user stopped it");
    assert!(matches!(error, ShellError::Cancelled), "{error}");
    assert!(
        started.elapsed() < Duration::from_secs(10),
        "stopped at once"
    );
}

#[test]
fn a_command_that_outlives_its_limit_is_reported_as_timed_out() {
    let temp = TempDir::new().expect("tempdir");
    let limit = Duration::from_millis(200);
    let error = run_until(LONG_COMMAND, temp.path(), limit, || false).expect_err("time limit");
    assert!(
        matches!(error, ShellError::TimedOut(reported) if reported == limit),
        "{error}"
    );
}

#[test]
fn each_ending_reads_as_what_happened_to_the_command() {
    assert_eq!(
        ShellError::Cancelled.report("sleep 30; echo done"),
        "cancelled: sleep 30; echo done"
    );
    assert_eq!(
        ShellError::TimedOut(Duration::from_secs(10)).report("sleep 12"),
        "stopped 'sleep 12': timed out after 10s (Esc stops it sooner)"
    );
    let launch = ShellError::Launch(io::Error::other("no shell"));
    assert_eq!(launch.report("true"), "cannot run 'true': no shell");
    assert!(std::error::Error::source(&launch).is_some());
    assert!(std::error::Error::source(&ShellError::Cancelled).is_none());
}

#[test]
fn a_run_in_a_deleted_folder_is_blamed_on_the_folder() {
    let temp = TempDir::new().expect("tempdir");
    let gone = temp.path().join("gone");
    let error = run_shell_command("echo hi", &gone).expect_err("nowhere to run");
    assert!(
        matches!(&error, ShellError::MissingFolder(folder) if *folder == gone),
        "{error:?}"
    );
    assert!(std::error::Error::source(&error).is_none());
}

/// A folder that is there does not become "missing" because something else
/// failed to start in it.
#[test]
fn only_a_folder_that_is_really_gone_is_reported_missing() {
    let temp = TempDir::new().expect("tempdir");
    let error = ShellError::launching_in(temp.path(), io::Error::other("no shell"));
    assert!(matches!(error, ShellError::Launch(_)), "{error:?}");
}
