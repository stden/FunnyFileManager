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
    let result = capture_with_timeout(command.current_dir(directory.path()), Duration::ZERO);
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
