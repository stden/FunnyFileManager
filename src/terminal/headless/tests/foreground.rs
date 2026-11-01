//! Purpose: Cancelling a job-control shell also stops its foreground job.

use super::*;
use std::{
    thread,
    time::{Duration, Instant},
};

#[test]
fn cancellation_stops_the_private_terminal_foreground_group() {
    let directory = tempfile::TempDir::new().unwrap();
    let pid_file = directory.path().join("foreground.pid");
    let host = HeadlessHost::new();
    let worker = foreground_fixture(&host, directory.path());
    start_input(&host);
    wait_for_pid(&pid_file);
    let pid = std::fs::read_to_string(&pid_file).unwrap().parse().unwrap();
    let pid = rustix::process::Pid::from_raw(pid).unwrap();
    host.cancel();
    assert_eq!(
        worker.join().unwrap().unwrap_err().kind(),
        io::ErrorKind::Interrupted
    );
    wait_stopped(pid);
}

fn foreground_fixture(
    host: &HeadlessHost,
    directory: &std::path::Path,
) -> thread::JoinHandle<io::Result<ExternalOutcome>> {
    let host = host.clone();
    let mut command = Command::new("sh");
    command
        .args([
            "-i",
            "-c",
            "sh -c 'read line; printf %s \"$$\" > foreground.pid; exec sleep 10'",
        ])
        .current_dir(directory);
    thread::spawn(move || {
        let _scope = host.enter();
        run_with_timeout(&command, Some(Duration::from_secs(15)))
    })
}

fn start_input(host: &HeadlessHost) {
    let startup = Instant::now() + Duration::from_secs(5);
    while host.snapshot().state != ExternalState::Running {
        assert!(Instant::now() < startup, "foreground shell never started");
        thread::sleep(Duration::from_millis(10));
    }
    host.input("\n").unwrap();
}

fn wait_stopped(pid: rustix::process::Pid) {
    let deadline = Instant::now() + Duration::from_secs(3);
    while running(pid) {
        assert!(
            Instant::now() < deadline,
            "foreground fixture survived cancellation"
        );
        thread::sleep(Duration::from_millis(10));
    }
}

fn running(pid: rustix::process::Pid) -> bool {
    if rustix::process::test_kill_process(pid).is_err() {
        return false;
    }
    let output = Command::new("ps")
        .args(["-o", "stat=", "-p", &pid.to_string()])
        .output()
        .unwrap();
    // Container PID 1 may leave a killed orphan zombie unreaped.
    let state = String::from_utf8_lossy(&output.stdout);
    !state.trim().is_empty() && !state.trim_start().starts_with('Z')
}

fn wait_for_pid(path: &std::path::Path) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !std::fs::read_to_string(path).is_ok_and(|value| !value.is_empty()) {
        assert!(
            Instant::now() < deadline,
            "foreground fixture never started"
        );
        thread::sleep(Duration::from_millis(10));
    }
}
