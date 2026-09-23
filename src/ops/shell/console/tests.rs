//! Exercise a local Codex stand-in with real Windows console handles.

use super::*;
use std::os::windows::io::AsRawHandle;
use tempfile::TempDir;
use windows_sys::Win32::{
    Foundation::WAIT_OBJECT_0,
    System::Threading::{TerminateProcess, WaitForSingleObject, STARTF_USESHOWWINDOW},
};

fn write_probe(directory: &Path) {
    std::fs::write(
        directory.join("codex.cmd"),
        concat!(
            "@echo off\r\n",
            "powershell.exe -NoLogo -NoProfile -NonInteractive -ExecutionPolicy Bypass ",
            "-File \"%~dp0probe.ps1\" %*\r\nexit\r\n"
        ),
    )
    .expect("local codex stand-in");
    std::fs::write(
        directory.join("probe.ps1"),
        r"$result = @{
    arguments = @($args)
    directory = [Environment]::CurrentDirectory
    inputRedirected = [Console]::IsInputRedirected
    outputRedirected = [Console]::IsOutputRedirected
    errorRedirected = [Console]::IsErrorRedirected
}
[IO.File]::WriteAllText((Join-Path $PSScriptRoot 'result.json'), ($result | ConvertTo-Json -Compress))
",
    )
    .expect("console probe");
}

fn wait_for_probe(process: &OwnedHandle) {
    // SAFETY: the owned handle stays live throughout these calls.
    let result = unsafe { WaitForSingleObject(process.as_raw_handle(), 15_000) };
    if result != WAIT_OBJECT_0 {
        unsafe { TerminateProcess(process.as_raw_handle(), 1) };
    }
    assert_eq!(result, WAIT_OBJECT_0, "console probe must finish");
}

fn assert_probe_result(directory: &Path) {
    let result: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(directory.join("result.json")).expect("probe output"),
    )
    .expect("probe JSON");
    let actual = Path::new(result["directory"].as_str().expect("working directory"));
    assert_eq!(
        actual.canonicalize().expect("actual directory"),
        directory.canonicalize().expect("expected directory")
    );
    assert_eq!(
        result["arguments"],
        serde_json::json!(["--model", "example", "two words", "Привет мир", "a & b"])
    );
    for stream in ["inputRedirected", "outputRedirected", "errorRedirected"] {
        assert_eq!(result[stream], false, "{stream}");
    }
}

#[test]
fn new_console_has_its_own_io_and_receives_arguments_and_directory() {
    let temp = TempDir::new().expect("tempdir");
    let directory = temp.path().join("Project with spaces");
    std::fs::create_dir(&directory).expect("project");
    write_probe(&directory);
    let command = ShellCommand::new(r#"codex --model example "two words" "Привет мир" "a & b""#)
        .expect("command");
    let mut startup = startup_info();
    // Allocate the same independent console without flashing a test window.
    startup.dwFlags |= STARTF_USESHOWWINDOW;
    startup.wShowWindow = 0; // SW_HIDE
    let process = spawn_with_startup(&command, &directory, &startup).expect("new console");
    wait_for_probe(&process);
    assert_probe_result(&directory);
}

#[test]
fn new_console_rejects_a_missing_working_directory() {
    let temp = TempDir::new().expect("tempdir");
    let command = ShellCommand::new("codex").expect("command");
    assert!(spawn(&command, &temp.path().join("missing")).is_err());
}

#[test]
fn canonical_windows_directory_is_preserved_when_launching_codex() {
    let temp = TempDir::new().expect("tempdir");
    let directory = temp.path().join("Project with spaces");
    std::fs::create_dir(&directory).expect("project");
    write_probe(&directory);
    let canonical = directory.canonicalize().expect("canonical directory");
    let probe = directory.join("codex.cmd");
    let input = format!(
        r#""{}" --model example "two words" "Привет мир" "a & b""#,
        probe.display()
    );
    let command = ShellCommand::new(&input).expect("command");
    let mut startup = startup_info();
    startup.dwFlags |= STARTF_USESHOWWINDOW;
    startup.wShowWindow = 0;
    let process = spawn_with_startup(&command, &canonical, &startup).expect("new console");
    wait_for_probe(&process);
    assert_probe_result(&directory);
}

#[test]
fn new_console_rejects_nul_instead_of_truncating_arguments() {
    let temp = TempDir::new().expect("tempdir");
    let command = ShellCommand::new("codex before\0after").expect("command");
    assert_eq!(
        spawn(&command, temp.path()).expect_err("NUL").kind(),
        io::ErrorKind::InvalidInput
    );
}
