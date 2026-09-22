//! Purpose: Contains Windows-specific behavior and diagnostic helpers.
//!
use crate::models::App;
#[cfg(any(test, windows))]
use crate::models::Viewer;
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
    app.set_overlay(crate::models::Overlay::Viewer(Viewer::report(
        title,
        report_lines(report),
    )));
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
mod tests;
