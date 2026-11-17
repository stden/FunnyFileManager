//! Purpose: Exercise real private PTYs with harmless fixture child processes.

use super::*;
use std::{
    thread,
    time::{Duration, Instant},
};

fn fixture_command(script: &str, directory: &std::path::Path) -> Command {
    #[cfg(windows)]
    let mut command = {
        let mut command = Command::new("powershell.exe");
        command.args(["-NoProfile", "-NonInteractive", "-Command", script]);
        command
    };
    #[cfg(not(windows))]
    let mut command = {
        let mut command = Command::new("sh");
        command.args(["-c", script]);
        command
    };
    command.current_dir(directory);
    command
}

#[test]
fn private_child_captures_output_and_exit_code() {
    let directory = tempfile::TempDir::new().unwrap();
    let host = HeadlessHost::new();
    let _scope = host.enter();
    let command = fixture_command("echo private-output; exit 7", directory.path());
    let outcome = run_with_timeout(&command, Some(Duration::from_secs(20))).unwrap();
    assert_eq!(outcome.code, 7);
    assert!(String::from_utf8_lossy(&outcome.output).contains("private-output"));
    assert_eq!(host.snapshot().state, ExternalState::Exited { code: 7 });
}

#[test]
fn remote_input_reaches_child_and_busy_reset_is_refused() {
    let directory = tempfile::TempDir::new().unwrap();
    let host = HeadlessHost::new();
    #[cfg(windows)]
    let script = "$line = [Console]::ReadLine(); Write-Output ('received:' + $line)";
    #[cfg(not(windows))]
    let script = "read line; printf 'received:%s\\n' \"$line\"";
    let command = fixture_command(script, directory.path());
    let worker_host = host.clone();
    let worker = thread::spawn(move || {
        let _scope = worker_host.enter();
        run_with_timeout(&command, Some(Duration::from_secs(20)))
    });
    wait_running(&host);
    assert_eq!(host.reset().unwrap_err().kind(), io::ErrorKind::WouldBlock);
    host.input("hello\r").unwrap();
    let result = worker.join().unwrap().unwrap();
    assert!(String::from_utf8_lossy(&result.output).contains("received:hello"));
}

fn wait_running(host: &HeadlessHost) {
    let deadline = Instant::now() + Duration::from_secs(15);
    while host.snapshot().state != ExternalState::Running {
        assert!(Instant::now() < deadline, "private child never started");
        thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn cancellation_stops_private_child() {
    let directory = tempfile::TempDir::new().unwrap();
    let host = HeadlessHost::new();
    #[cfg(windows)]
    let script = "Start-Sleep -Seconds 60";
    #[cfg(not(windows))]
    let script = "sleep 60";
    let command = fixture_command(script, directory.path());
    let worker_host = host.clone();
    let worker = thread::spawn(move || {
        let _scope = worker_host.enter();
        run(&command)
    });
    wait_running(&host);
    host.cancel();
    assert_eq!(
        worker.join().unwrap().unwrap_err().kind(),
        io::ErrorKind::Interrupted
    );
    assert_eq!(host.snapshot().state, ExternalState::Cancelled);
}

#[test]
fn a_running_child_observes_viewport_resize() {
    let directory = tempfile::TempDir::new().unwrap();
    let host = HeadlessHost::new();
    host.resize(100, 30).unwrap();
    #[cfg(windows)]
    let script = "Write-Output 'resize-ready'; while ([Console]::WindowWidth -ne 90 -or [Console]::WindowHeight -ne 35) { Start-Sleep -Milliseconds 10 }; Write-Output 'resize-applied'";
    #[cfg(not(windows))]
    let script = "printf 'resize-ready\\n'; while [ \"$(stty size)\" != '35 90' ]; do sleep 0.01; done; printf 'resize-applied\\n'";
    let command = fixture_command(script, directory.path());
    let worker_host = host.clone();
    let worker = thread::spawn(move || {
        let _scope = worker_host.enter();
        run_with_timeout(&command, Some(Duration::from_secs(20)))
    });
    let deadline = Instant::now() + Duration::from_secs(15);
    while !host.snapshot().output.contains("resize-ready") {
        assert!(
            Instant::now() < deadline,
            "private child never reached resize"
        );
        thread::sleep(Duration::from_millis(10));
    }
    host.resize(90, 35).unwrap();
    let output = worker.join().unwrap().unwrap();
    assert!(String::from_utf8_lossy(&output.output).contains("resize-applied"));
}

#[cfg(unix)]
#[test]
fn unix_capture_rearms_unread_output_larger_than_one_chunk() {
    let directory = tempfile::TempDir::new().unwrap();
    let host = HeadlessHost::new();
    let _scope = host.enter();
    let command = fixture_command("head -c 131072 /dev/zero | tr '\\000' x", directory.path());
    let outcome = run_with_timeout(&command, Some(Duration::from_secs(20))).unwrap();
    assert_eq!(outcome.code, 0);
    assert_eq!(outcome.output.len(), 131_072);
    assert!(outcome.output.iter().all(|&byte| byte == b'x'));
}

#[cfg(windows)]
#[test]
fn canonical_windows_cwd_preserves_directory_and_relative_file_access() {
    let root = tempfile::TempDir::new().unwrap();
    let directory = root.path().join("cwd с пробелами");
    std::fs::create_dir(&directory).unwrap();
    let canonical = directory.canonicalize().unwrap();
    std::fs::write(
        directory.join("relative-fixture.txt"),
        "ffm-relative-cwd-proof",
    )
    .unwrap();
    let host = HeadlessHost::new();
    host.resize(240, 40).unwrap();
    let _scope = host.enter();
    let mut command = Command::new("cmd.exe");
    command
        .args(["/D", "/C", "cd & type relative-fixture.txt"])
        .current_dir(&canonical);
    let outcome = run_with_timeout(&command, Some(Duration::from_secs(20))).unwrap();
    assert_eq!(outcome.code, 0);
    let output = String::from_utf8_lossy(&outcome.output);
    assert_windows_cwd(&output, &canonical);
    assert!(
        output.contains("ffm-relative-cwd-proof"),
        "relative input: {output}"
    );
}

#[cfg(windows)]
fn assert_windows_cwd(output: &str, canonical: &std::path::Path) {
    let expected = crate::windows::console_directory(canonical)
        .display()
        .to_string();
    assert!(
        output
            .lines()
            .filter_map(|line| crate::console::first_drawable(std::iter::once(line)))
            .any(|line| line.trim() == expected),
        "child cwd: {output}"
    );
}
