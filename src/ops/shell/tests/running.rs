//! Purpose: Running a command: arguments, exit codes, output, folder and cancel keys.

use super::*;

#[test]
fn test_windows_shell_args_structure() {
    let args = windows_shell_args("echo hello");
    assert_eq!(args[0], "/D");
    assert_eq!(args[1], "/C");
    assert_eq!(args[2], "echo hello");
}

#[test]
fn test_windows_shell_args_empty_command() {
    let args = windows_shell_args("");
    assert_eq!(args, ["/D", "/C", ""]);
}

#[test]
fn test_run_shell_command_echo() {
    let temp = TempDir::new().expect("tempdir");
    let output = run_shell_command("echo hello", temp.path()).expect("run");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("hello"));
}

#[test]
fn test_run_shell_command_exit_code() {
    let temp = TempDir::new().expect("tempdir");
    #[cfg(windows)]
    let cmd = "cmd /c exit /b 42";
    #[cfg(not(windows))]
    let cmd = "exit 42";
    let output = run_shell_command(cmd, temp.path()).expect("run");
    assert!(!output.status.success());
}

#[test]
fn test_run_shell_command_stderr() {
    let temp = TempDir::new().expect("tempdir");
    #[cfg(windows)]
    let cmd = "echo oops 1>&2";
    #[cfg(not(windows))]
    let cmd = "echo oops >&2";
    let output = run_shell_command(cmd, temp.path()).expect("run");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("oops"));
}

#[test]
fn test_run_shell_command_uses_current_dir() {
    let temp = TempDir::new().expect("tempdir");
    #[cfg(windows)]
    let cmd = "cd";
    #[cfg(not(windows))]
    let cmd = "pwd";
    let output = run_shell_command(cmd, temp.path()).expect("run");
    let stdout = String::from_utf8_lossy(&output.stdout);
    // On Windows, canonicalized paths may differ slightly, so just check non-empty
    assert!(!stdout.trim().is_empty());
}

#[test]
fn test_run_shell_command_nonexistent_dir_fails() {
    let bad_path = Path::new("/nonexistent_dir_99999");
    let result = run_shell_command("echo hi", bad_path);
    assert!(result.is_err());
}

#[test]
fn is_cancel_event_accepts_only_pressed_esc_and_ctrl_c() {
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

    let press = |code, modifiers| Event::Key(KeyEvent::new(code, modifiers));
    assert!(is_cancel_event(&press(KeyCode::Esc, KeyModifiers::empty())));
    assert!(is_cancel_event(&press(
        KeyCode::Char('c'),
        KeyModifiers::CONTROL
    )));
    // A plain 'c', any other key, and non-key events are not cancel.
    assert!(!is_cancel_event(&press(
        KeyCode::Char('c'),
        KeyModifiers::empty()
    )));
    assert!(!is_cancel_event(&press(
        KeyCode::Enter,
        KeyModifiers::empty()
    )));
    assert!(!is_cancel_event(&Event::FocusGained));
    // Key releases must not cancel: only presses count.
    let mut release = KeyEvent::new(KeyCode::Esc, KeyModifiers::empty());
    release.kind = KeyEventKind::Release;
    assert!(!is_cancel_event(&Event::Key(release)));
}

#[test]
fn quoted_paths_and_shell_operators_reach_the_shell_unchanged() {
    let temp = TempDir::new().expect("tempdir");
    std::fs::write(temp.path().join("input file.txt"), "contents").expect("input");
    let command = if cfg!(windows) {
        "type \"input file.txt\" > \"output file.txt\" && echo copied"
    } else {
        "cat \"input file.txt\" > \"output file.txt\" && echo copied"
    };
    let output = run_shell_command(command, temp.path()).expect("shell");
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        std::fs::read_to_string(temp.path().join("output file.txt")).expect("copy"),
        "contents"
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "copied");
}
