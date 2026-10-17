//! Purpose: Timeouts that kill the whole group, backgrounded children, and large output.

use super::*;

/// A command that never exits must be killed, not block the event loop forever.
#[test]
fn test_run_shell_command_times_out_instead_of_hanging() {
    let temp = TempDir::new().expect("tempdir");
    #[cfg(windows)]
    let cmd = "ping -n 30 127.0.0.1 >nul";
    #[cfg(not(windows))]
    let cmd = "sleep 30";

    let started = Instant::now();
    let error = run_shell_command_with_timeout(cmd, temp.path(), Duration::from_millis(200))
        .expect_err("must not run to completion");

    assert!(matches!(error, ShellError::TimedOut(_)), "{error}");
    assert!(
        started.elapsed() < Duration::from_secs(10),
        "gave up after {:?}, so the wait was not bounded",
        started.elapsed()
    );
}

/// Is `pid` a live process (a zombie waiting to be reaped does not count)?
///
/// `kill(pid, 0)` alone cannot tell the two apart — signalling a zombie still
/// succeeds — so the process state comes from `/proc`, which is why this is
/// Linux-only.
#[cfg(target_os = "linux")]
fn is_alive(pid: i32) -> bool {
    let Ok(stat) = std::fs::read_to_string(format!("/proc/{pid}/stat")) else {
        return false; // already reaped
    };
    // "pid (comm) state …" — comm may contain spaces and parentheses, so read
    // the state after the last ')'.
    let state = stat
        .rsplit_once(')')
        .and_then(|(_, rest)| rest.split_whitespace().next());
    state != Some("Z")
}

/// A timed-out command must take its grandchildren with it. `sh` is spawned as
/// its own process group leader, so the timeout can SIGKILL the whole group;
/// killing the shell alone left the backgrounded `sleep` alive, reparented to
/// init and still holding the pipe write end — which is what used to wedge the
/// drain threads for good.
#[cfg(target_os = "linux")]
#[test]
fn test_timeout_kills_the_whole_process_group() {
    let temp = TempDir::new().expect("tempdir");
    let pidfile = temp.path().join("child.pid");
    // `$!` is the *grandchild*: the process that only a group-wide kill reaches.
    // `wait` keeps the shell itself alive so the run really does time out.
    let command = format!("sleep 120 & echo $! > {}; wait", pidfile.display());

    let error = run_shell_command_with_timeout(&command, temp.path(), Duration::from_millis(300))
        .expect_err("must time out");
    assert!(matches!(error, ShellError::TimedOut(_)), "{error}");

    let grandchild: i32 = std::fs::read_to_string(&pidfile)
        .expect("the shell wrote the background pid")
        .trim()
        .parse()
        .expect("pid is a number");

    // Signal delivery and reaping are asynchronous, so poll rather than assert
    // once — but bounded, this must not become another unbounded wait.
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if !is_alive(grandchild) {
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    // Do not leave a stray `sleep` behind if the assertion fails.
    if let Some(pid) = rustix::process::Pid::from_raw(grandchild) {
        if let Err(error) = rustix::process::kill_process(pid, rustix::process::Signal::KILL) {
            eprintln!("could not kill stray grandchild {grandchild}: {error}");
        }
    }
    panic!("grandchild {grandchild} outlived the timeout — the group was not killed");
}

/// A backgrounded grandchild inherits the pipe write end, so `read_to_end`
/// cannot reach EOF until *it* dies. The drain must therefore be bounded by the
/// same wall-clock budget as the wait loop: `sh -c 'sleep 30 &'` exits in
/// milliseconds, and before this was fixed the join blocked the event loop for
/// the full 30s with the terminal in raw mode.
#[cfg(not(windows))]
#[test]
fn test_run_shell_command_backgrounded_grandchild_does_not_block_the_drain() {
    let temp = TempDir::new().expect("tempdir");
    let started = Instant::now();
    let output = run_shell_command_with_timeout(
        "echo bye; sleep 30 &",
        temp.path(),
        Duration::from_millis(300),
    )
    .expect("the shell itself exited, so this is not a timeout");

    assert!(output.status.success());
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "drain was not bounded: took {:?}",
        started.elapsed()
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("bye"),
        "output written before the deadline must still be reported"
    );
}

/// Output larger than the OS pipe buffer must not deadlock the wait loop.
#[cfg(not(windows))]
#[test]
fn test_run_shell_command_survives_output_larger_than_pipe_buffer() {
    let temp = TempDir::new().expect("tempdir");
    // ~290 KiB, far past the 64 KiB Linux pipe buffer.
    let output =
        run_shell_command_with_timeout("seq 1 50000", temp.path(), Duration::from_secs(30))
            .expect("run");
    assert!(output.status.success());
    assert!(output.stdout.len() > 100_000, "output was truncated");
}
