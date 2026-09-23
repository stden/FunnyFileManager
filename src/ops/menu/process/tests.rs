use super::*;

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
