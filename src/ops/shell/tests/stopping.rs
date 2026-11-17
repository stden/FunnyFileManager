//! Purpose: How a command is stopped: SIGHUP first, so it can clean up, and
//! SIGKILL for a command that will not go.

use super::*;

/// A shell told to hang up runs its HUP trap: the command got the chance to
/// clean up that a shell gives its jobs when the terminal goes away.
#[test]
fn a_stopped_command_hears_the_hang_up_first() {
    let temp = TempDir::new().expect("tempdir");
    let command = "trap 'echo cleaned > hung-up; exit 1' HUP; sleep 30 & wait";

    let error = run_shell_command_with_timeout(command, temp.path(), Duration::from_millis(300))
        .expect_err("must time out");

    assert!(matches!(error, ShellError::TimedOut(_)), "{error}");
    let note = std::fs::read_to_string(temp.path().join("hung-up")).expect("the trap ran");
    assert_eq!(note.trim(), "cleaned");
}

/// A command that ignores SIGHUP still stops, within the grace period and
/// not the command's own 30 seconds.
#[test]
fn a_command_that_ignores_the_hang_up_is_killed() {
    let temp = TempDir::new().expect("tempdir");
    let started = Instant::now();

    let error = run_shell_command_with_timeout(
        "trap '' HUP; sleep 30",
        temp.path(),
        Duration::from_millis(300),
    )
    .expect_err("must time out");

    assert!(matches!(error, ShellError::TimedOut(_)), "{error}");
    assert!(
        started.elapsed() < Duration::from_secs(10),
        "the stop waited {:?}, so the kill never came",
        started.elapsed()
    );
}
