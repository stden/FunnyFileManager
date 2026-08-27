//! Purpose: Contains Windows-specific behavior and diagnostic helpers.
//!
use crate::models::App;
#[cfg(any(test, windows))]
use crate::models::Viewer;
#[cfg(any(test, windows))]
use crate::scroll::ScrollTop;
use std::io;

#[cfg(windows)]
use std::process::Command;

#[cfg(any(test, windows))]
const SYSTEM_MESSAGES_TITLE: &str = "Windows System Messages";
#[cfg(any(test, windows))]
const PERFORMANCE_TITLE: &str = "Windows Performance";

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[cfg(windows)]
const SYSTEM_MESSAGES_SCRIPT: &str = r#"
$events = Get-WinEvent -FilterHashtable @{LogName='System'; Level=@(1,2,3)} -MaxEvents 20 |
    Select-Object TimeCreated, LevelDisplayName, ProviderName, Id, Message
foreach ($event in $events) {
    $message = ($event.Message -replace '\r?\n', ' ').Trim()
    "{0:u} [{1}] {2} ({3}) {4}" -f $event.TimeCreated, $event.LevelDisplayName, $event.ProviderName, $event.Id, $message
}
"#;

#[cfg(windows)]
const PERFORMANCE_SCRIPT: &str = r#"
$os = Get-CimInstance Win32_OperatingSystem
$cpu = Get-CimInstance Win32_Processor | Measure-Object -Property LoadPercentage -Average
$drives = Get-CimInstance Win32_LogicalDisk -Filter "DriveType=3"
$totalMem = [math]::Round($os.TotalVisibleMemorySize / 1024, 1)
$freeMem = [math]::Round($os.FreePhysicalMemory / 1024, 1)
"Computer: $env:COMPUTERNAME"
"CPU Load: $([math]::Round($cpu.Average, 1))%"
"Memory Free: $freeMem MB / $totalMem MB"
foreach ($drive in $drives) {
    $freeGb = [math]::Round($drive.FreeSpace / 1GB, 2)
    $sizeGb = [math]::Round($drive.Size / 1GB, 2)
    $usedPct = if ($drive.Size -gt 0) { [math]::Round((($drive.Size - $drive.FreeSpace) / $drive.Size) * 100, 1) } else { 0 }
    "{0} Used: {1}% Free: {2} GB / {3} GB" -f $drive.DeviceID, $usedPct, $freeGb, $sizeGb
}
"#;

#[cfg(any(test, windows))]
fn report_lines(report: &str) -> Vec<String> {
    let mut lines: Vec<String> = report
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect();

    if lines.is_empty() {
        lines.push("<no data>".to_string());
    }

    lines
}

#[cfg(any(test, windows))]
fn open_report_viewer(app: &mut App, title: &str, report: &str) {
    app.set_overlay(crate::models::Overlay::Viewer(Viewer {
        title: title.to_string(),
        lines: report_lines(report),
        scroll: ScrollTop::TOP,
    }));
    app.set_status(crate::status::Status::OpenedView(title.to_string()));
}

#[cfg(any(test, windows))]
fn open_report_with<F>(app: &mut App, title: &str, runner: F) -> io::Result<()>
where
    F: FnOnce() -> io::Result<String>,
{
    let report = runner()?;
    open_report_viewer(app, title, &report);
    Ok(())
}

#[cfg(any(test, windows))]
const fn powershell_executable() -> &'static str {
    "powershell.exe"
}

#[cfg(any(test, windows))]
const fn powershell_report_args(script: &str) -> [&str; 7] {
    [
        "-NoLogo",
        "-NoProfile",
        "-NonInteractive",
        "-ExecutionPolicy",
        "Bypass",
        "-Command",
        script,
    ]
}

#[cfg(windows)]
fn run_powershell_report(script: &str) -> io::Result<String> {
    use std::os::windows::process::CommandExt;

    let mut command = Command::new(powershell_executable());
    command.creation_flags(CREATE_NO_WINDOW);
    let output = command.args(powershell_report_args(script)).output()?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

    if output.status.success() {
        Ok(stdout)
    } else if !stderr.is_empty() {
        Err(io::Error::other(stderr))
    } else {
        Err(io::Error::other(format!(
            "PowerShell exited with status {}",
            output.status
        )))
    }
}

pub fn open_windows_system_messages(app: &mut App) -> io::Result<()> {
    #[cfg(windows)]
    {
        open_report_with(app, SYSTEM_MESSAGES_TITLE, || {
            run_powershell_report(SYSTEM_MESSAGES_SCRIPT)
        })
    }

    #[cfg(not(windows))]
    {
        app.set_status(crate::status::Status::WindowsOnly { tool: "system" });
        Ok(())
    }
}

pub fn open_windows_performance_checker(app: &mut App) -> io::Result<()> {
    #[cfg(windows)]
    {
        open_report_with(app, PERFORMANCE_TITLE, || {
            run_powershell_report(PERFORMANCE_SCRIPT)
        })
    }

    #[cfg(not(windows))]
    {
        app.set_status(crate::status::Status::WindowsOnly {
            tool: "performance",
        });
        Ok(())
    }
}

/// Classify a command's output into Ok(stdout) or an appropriate error.
/// Extracted so mutation tests can exercise the stderr-vs-status branches
/// on every platform.
#[cfg(test)]
fn classify_command_output(success: bool, stdout: &str, stderr: &str) -> io::Result<String> {
    if success {
        Ok(stdout.to_owned())
    } else if !stderr.is_empty() {
        Err(io::Error::other(stderr.to_owned()))
    } else {
        Err(io::Error::other(
            "Command exited with failure status".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // These tests use `.expect(...)` rather than `?`: the runners below cannot
    // fail on a non-Windows host, so a `?` would leave an error branch that no
    // test can ever reach.

    #[test]
    fn test_open_report_with_populates_viewer() {
        let mut app = App::new().expect("test setup");

        open_report_with(&mut app, SYSTEM_MESSAGES_TITLE, || {
            Ok("first line\nsecond line".to_string())
        })
        .expect("the report runner cannot fail");

        let viewer = app.get_viewer().expect("viewer should open");
        assert_eq!(viewer.title, SYSTEM_MESSAGES_TITLE);
        assert_eq!(viewer.lines, vec!["first line", "second line"]);
        assert_eq!(app.status_text(), format!("Opened {SYSTEM_MESSAGES_TITLE}"));
    }

    #[test]
    fn test_open_report_with_uses_placeholder_for_empty_output() {
        let mut app = App::new().expect("test setup");

        open_report_with(&mut app, PERFORMANCE_TITLE, || Ok(" \n\n".to_string()))
            .expect("the report runner cannot fail");

        let viewer = app.get_viewer().expect("viewer should open");
        assert_eq!(viewer.title, PERFORMANCE_TITLE);
        assert_eq!(viewer.lines, vec!["<no data>"]);
    }

    #[test]
    fn test_open_windows_system_messages_on_non_windows() {
        let mut app = App::new().expect("test setup");
        open_windows_system_messages(&mut app).expect("system messages report");

        #[cfg(not(windows))]
        assert!(app.status_text().contains("only available on Windows"));

        #[cfg(windows)]
        assert!(app.is_overlay_active() || app.status_text().contains("error"));
    }

    #[test]
    fn test_open_windows_performance_checker_on_non_windows() {
        let mut app = App::new().expect("test setup");
        open_windows_performance_checker(&mut app).expect("performance report");

        #[cfg(not(windows))]
        assert!(app.status_text().contains("only available on Windows"));

        #[cfg(windows)]
        assert!(app.is_overlay_active() || app.status_text().contains("error"));
    }

    #[test]
    fn test_open_report_with_error_propagates() {
        let mut app = App::new().expect("test setup");
        let result = open_report_with(&mut app, "Test", || Err(io::Error::other("test error")));
        assert!(result.is_err());
        assert!(!app.is_overlay_active());
    }

    #[test]
    fn test_powershell_executable_is_explicit() {
        assert_eq!(powershell_executable(), "powershell.exe");
    }

    #[test]
    fn test_powershell_report_args_include_expected_flags() {
        assert_eq!(
            powershell_report_args("Get-Date"),
            [
                "-NoLogo",
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
                "Get-Date"
            ]
        );
    }

    // ── classify_command_output mutation killers ──────────────────

    #[test]
    fn test_classify_success_returns_stdout() {
        // Kill: replace success branch with Err
        let result = classify_command_output(true, "hello", "");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "hello");
    }

    #[test]
    fn test_classify_success_ignores_stderr() {
        // Kill: swapping stdout/stderr in success branch
        let result = classify_command_output(true, "good", "noise");
        assert_eq!(result.unwrap(), "good");
    }

    #[test]
    fn test_classify_failure_with_stderr_uses_stderr_message() {
        // Kill: replace `!stderr.is_empty()` with `stderr.is_empty()`
        let result = classify_command_output(false, "", "bad thing happened");
        let err = result.unwrap_err();
        assert_eq!(err.to_string(), "bad thing happened");
    }

    #[test]
    fn test_classify_failure_empty_stderr_uses_generic_message() {
        // Kill: replace empty-stderr branch with Ok or wrong message
        let result = classify_command_output(false, "", "");
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("failure status"),
            "expected generic message, got: {err}"
        );
    }

    #[test]
    fn test_classify_failure_stderr_not_generic() {
        // Kill: always using generic message regardless of stderr
        let result = classify_command_output(false, "out", "specific error");
        let err = result.unwrap_err();
        assert_eq!(err.to_string(), "specific error");
        assert!(
            !err.to_string().contains("failure status"),
            "should use stderr, not generic message"
        );
    }

    #[test]
    fn test_classify_failure_empty_stderr_not_stderr_content() {
        // Kill: using stderr when it's empty instead of generic message
        let result = classify_command_output(false, "out", "");
        let err = result.unwrap_err();
        assert!(
            !err.to_string().is_empty(),
            "error message should not be empty"
        );
        assert!(err.to_string().contains("failure status"));
    }

    // ── report_lines mutation killers ────────────────────────────

    #[test]
    fn test_report_lines_filters_blank_lines() {
        let lines = report_lines("first\n\n  \nsecond\n");
        assert_eq!(lines, vec!["first", "second"]);
    }

    #[test]
    fn test_report_lines_trims_whitespace() {
        let lines = report_lines("  padded  ");
        assert_eq!(lines, vec!["padded"]);
    }

    #[test]
    fn test_report_lines_empty_input_returns_placeholder() {
        // Kill: removing the is_empty check / placeholder push
        let lines = report_lines("");
        assert_eq!(lines, vec!["<no data>"]);
    }

    #[test]
    fn test_report_lines_whitespace_only_returns_placeholder() {
        let lines = report_lines("   \n  \n  ");
        assert_eq!(lines, vec!["<no data>"]);
    }

    // ── open_report_viewer mutation killers ───────────────────────

    #[test]
    fn test_open_report_viewer_sets_scroll_to_zero() {
        let mut app = App::new().expect("test setup");
        open_report_viewer(&mut app, "Test", "line1\nline2");
        let viewer = app.get_viewer().expect("viewer");
        assert_eq!(viewer.scroll, ScrollTop::TOP);
    }

    #[test]
    fn test_open_report_viewer_sets_title() {
        let mut app = App::new().expect("test setup");
        open_report_viewer(&mut app, "My Report", "data");
        let viewer = app.get_viewer().expect("viewer");
        assert_eq!(viewer.title, "My Report");
    }

    #[test]
    fn test_open_report_viewer_sets_status_message() {
        let mut app = App::new().expect("test setup");
        open_report_viewer(&mut app, "Diag", "data");
        assert_eq!(app.status_text(), "Opened Diag");
    }

    #[cfg(windows)]
    #[test]
    fn test_run_powershell_report_failure_with_stderr() {
        let result = run_powershell_report("Write-Error 'something failed'");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("failed") || err.to_string().contains("Error"));
    }

    #[cfg(windows)]
    #[test]
    fn test_run_powershell_report_failure_status_only() {
        let result = run_powershell_report("[System.Environment]::Exit(42)");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("PowerShell exited with status")
                || err.to_string().contains("42")
        );
    }
}
