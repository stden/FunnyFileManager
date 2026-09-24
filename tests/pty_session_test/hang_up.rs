//! Purpose: A terminal that hangs up ends ffm, and whatever it started, cleanly.

use super::*;

/// Hanging up while a `:` command runs: `isatty` on a hung-up terminal fails,
/// which used to read as "not a terminal, leave it to crossterm". crossterm
/// then read the dead terminal forever at 100% CPU, the command's timeout
/// never came round, and its process group outlived ffm.
#[test]
fn a_terminal_that_hangs_up_during_a_shell_command_stops_it_and_ends_ffm() {
    let mut ffm = Ffm::start();
    let pid_file = ffm.work().join("shell.pid");
    ffm.type_keys(b":echo $$ > shell.pid; sleep 30\r");
    let started = ffm.wait_for(PATIENCE, |_| read_group(&pid_file).is_some());
    let group = started
        .and_then(|_| read_group(&pid_file))
        .expect("the shell never started");

    ffm.hang_up();
    let status = ffm.wait_for_exit();
    let group_gone = ffm.wait_for(HANG_UP, |_| test_kill_process_group(group).is_err());
    // A failed run must not leave the command behind either. After a passing
    // one the group is already gone, and the kill fails as it should.
    #[expect(
        clippy::let_underscore_must_use,
        reason = "best-effort cleanup: either outcome is fine; drop() of a Copy result is a no-op"
    )]
    let _ = kill_process_group(group, Signal::KILL);

    assert_ended_cleanly(status);
    assert!(group_gone.is_some(), "the shell command outlived ffm");
}

/// Waiting for a key, ffm sits in `poll(2)`, which sees the hang-up. What
/// followed was the failure: restoring the dead terminal fails, and printing
/// that failure onto it panicked.
#[test]
fn a_terminal_that_hangs_up_while_ffm_waits_for_a_key_ends_it_cleanly() {
    let mut ffm = Ffm::start();

    ffm.hang_up();

    assert_ended_cleanly(ffm.wait_for_exit());
}

#[expect(
    clippy::expect_used,
    reason = "an assertion helper: failing the test with this message is its job"
)]
fn assert_ended_cleanly(status: Option<ExitStatus>) {
    let status = status.expect("ffm kept running after its terminal hung up");
    assert!(
        status.success(),
        "a hang-up ends the session the way F10 does, not as a failure: {status}"
    );
}

/// The process group the shell wrote down: `$$` of `sh -c`, which ffm starts
/// as the leader of its own group.
fn read_group(path: &Path) -> Option<Pid> {
    let text = fs::read_to_string(path).ok()?;
    Pid::from_raw(text.trim().parse().ok()?)
}
