use super::*;

#[test]
fn cancellation_stops_a_captured_command_without_reporting_a_timeout() {
    let temp = tempfile::TempDir::new().expect("temporary directory");
    let mut command = sleep_command();
    let error = capture_cancellable(&mut command, Duration::from_secs(60), temp.path(), || true)
        .expect_err("cancelled");
    assert_eq!(error.kind(), io::ErrorKind::Interrupted);
    assert!(error.to_string().starts_with("Command cancelled"));
}

#[cfg(windows)]
fn sleep_command() -> Command {
    let mut command = Command::new("powershell.exe");
    command.args([
        "-NoProfile",
        "-NonInteractive",
        "-Command",
        "Start-Sleep -Seconds 20",
    ]);
    command
}

#[cfg(not(windows))]
fn sleep_command() -> Command {
    let mut command = Command::new("sleep");
    command.arg("20");
    command
}

#[test]
fn timed_out_commands_are_terminated_and_reaped() -> io::Result<()> {
    let directory = tempfile::TempDir::new()?;
    let mut command = if cfg!(windows) {
        let mut command = Command::new("powershell.exe");
        command.args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "Start-Sleep -Seconds 5",
        ]);
        command
    } else {
        let mut command = Command::new("sleep");
        command.arg("5");
        command
    };
    let result = capture_with_timeout(
        command.current_dir(directory.path()),
        Duration::ZERO,
        &std::env::temp_dir(),
    );
    assert_eq!(result.unwrap_err().kind(), io::ErrorKind::TimedOut);
    Ok(())
}

#[test]
fn oversized_completed_output_is_rejected_without_truncation() -> io::Result<()> {
    let mut file = tempfile::tempfile()?;
    file.set_len(MAX_OUTPUT + 1)?;
    assert!(read_output(&mut file).is_err());
    Ok(())
}

#[test]
fn capture_preserves_exit_status_and_both_streams() -> io::Result<()> {
    let directory = tempfile::TempDir::new()?;
    let mut command = if cfg!(windows) {
        let mut command = Command::new("cmd");
        command.args(["/D", "/C", "echo output & echo problem 1>&2 & exit /b 7"]);
        command
    } else {
        let mut command = Command::new("sh");
        command.args(["-c", "echo output; echo problem >&2; exit 7"]);
        command
    };
    let result = capture(command.current_dir(directory.path()))?;
    assert_eq!(result.status.code(), Some(7));
    assert!(String::from_utf8_lossy(&result.stdout).contains("output"));
    assert!(String::from_utf8_lossy(&result.stderr).contains("problem"));
    Ok(())
}

/// A temp directory that is missing or read-only is named as the fault: the
/// OS error names no path, and it must not read as a program that failed to
/// start. Nothing runs.
#[cfg(unix)]
#[test]
fn a_missing_temp_directory_is_named_and_nothing_runs() -> io::Result<()> {
    let directory = tempfile::TempDir::new()?;
    let missing = directory.path().join("nonexistent-tmp");
    let mut command = Command::new("sh");
    command
        .args(["-c", "touch ran"])
        .current_dir(directory.path());

    let error = capture_with_timeout(&mut command, Duration::from_mins(1), &missing)
        .expect_err("no temp directory");

    let text = error.to_string();
    assert!(
        text.starts_with(&format!(
            "cannot create a temporary file in {}: ",
            missing.display()
        )),
        "{text}"
    );
    assert_eq!(error.kind(), io::ErrorKind::NotFound);
    assert!(!directory.path().join("ran").exists(), "nothing ran");
    Ok(())
}

/// A program that is not there is named, since the OS error does not say
/// which one it could not find. (Its kind is the OS's: `NotFound`, or
/// `PermissionDenied` when a `PATH` entry cannot be searched.)
#[test]
fn a_missing_program_is_named() {
    let mut command = Command::new("ffm-no-such-program");

    let error = capture(&mut command).expect_err("not installed");

    assert!(
        error
            .to_string()
            .starts_with("cannot run ffm-no-such-program: "),
        "{error}"
    );
}

/// A kill or a reap that fails is said, not dropped.
#[test]
fn a_failed_stop_step_is_named() {
    let error = io::Error::other("no such process");
    assert_eq!(
        failed("stopping it", Err(error)).as_deref(),
        Some("stopping it failed: no such process")
    );
    assert_eq!(failed("waiting for it", Ok(())), None);
}

/// Review finding: a stop at the time limit killed only the direct child.
/// An npm or Volta shim is that child, and the real CLI under it ran on. The
/// child now leads a process group, and the whole group is stopped.
#[cfg(target_os = "linux")]
#[test]
fn a_timeout_stops_what_the_command_started_too() -> io::Result<()> {
    let directory = tempfile::TempDir::new()?;
    let pid_file = directory.path().join("grandchild");
    let mut command = Command::new("sh");
    command
        .args(["-c", "sleep 60 & echo $! > \"$0\"; wait"])
        .arg(&pid_file)
        .current_dir(directory.path());
    let result = capture_with_timeout(&mut command, Duration::from_secs(2), &std::env::temp_dir());
    assert_eq!(result.unwrap_err().kind(), io::ErrorKind::TimedOut);
    let pid = std::fs::read_to_string(&pid_file)?;
    let stat = format!("/proc/{}/stat", pid.trim());
    // Killed, then reaped by init: gone, or at worst a zombie waiting for it.
    let stopped = || {
        std::fs::read_to_string(&stat)
            .map_or(true, |line| line.split_whitespace().nth(2) == Some("Z"))
    };
    let deadline = Instant::now() + Duration::from_secs(5);
    while !stopped() && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(20));
    }
    assert!(stopped(), "the grandchild {} still runs", pid.trim());
    Ok(())
}

/// A child that leads no group of its own is still stopped: the group kill
/// finds nothing, and the child itself is killed instead.
#[cfg(unix)]
#[test]
fn a_child_outside_a_group_of_its_own_is_still_stopped() -> io::Result<()> {
    let mut child = Command::new("sleep").arg("30").spawn()?;
    Tree::of(&child).kill(&mut child)?;
    assert!(!child.wait()?.success());
    Ok(())
}
