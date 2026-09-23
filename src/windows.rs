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

/// Own an LSP process tree so a crashed or unresponsive server cannot orphan children.
#[cfg(windows)]
#[allow(unsafe_code)]
pub(crate) fn lsp_job(
    child: &std::process::Child,
) -> io::Result<std::os::windows::io::OwnedHandle> {
    use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
    use windows_sys::Win32::System::JobObjects::{AssignProcessToJobObject, CreateJobObjectW};
    // SAFETY: null security/name create a private, non-inherited job handle.
    let raw = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
    if raw.is_null() {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: CreateJobObjectW returned a unique live handle; ownership transfers once.
    let job = unsafe { OwnedHandle::from_raw_handle(raw) };
    configure_lsp_job(raw)?;
    // SAFETY: both handles remain live throughout the call; the child is owned by the caller.
    if unsafe { AssignProcessToJobObject(raw, child.as_raw_handle()) } == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(job)
}

#[cfg(windows)]
#[allow(unsafe_code)]
fn configure_lsp_job(handle: windows_sys::Win32::Foundation::HANDLE) -> io::Result<()> {
    use windows_sys::Win32::System::JobObjects::{
        JobObjectExtendedLimitInformation, SetInformationJobObject,
        JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    };
    let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
    limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
    let size = u32::try_from(std::mem::size_of_val(&limits)).map_err(io::Error::other)?;
    // SAFETY: limits points to an initialized structure of the supplied size for this class.
    let result = unsafe {
        SetInformationJobObject(
            handle,
            JobObjectExtendedLimitInformation,
            std::ptr::from_ref(&limits).cast(),
            size,
        )
    };
    if result == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(windows)]
const SYSTEM_MESSAGES_SCRIPT: &str = include_str!("windows/system_messages.ps1");

#[cfg(windows)]
const PERFORMANCE_SCRIPT: &str = include_str!("windows/performance.ps1");

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
/// Convert filesystem paths to the ordinary spelling understood by cmd.exe.
#[cfg(windows)]
pub(crate) fn console_directory(path: &std::path::Path) -> std::path::PathBuf {
    use std::{
        ffi::OsString,
        os::windows::ffi::{OsStrExt, OsStringExt},
        path::{Component, Prefix},
    };
    let Some(Component::Prefix(prefix)) = path.components().next() else {
        return path.to_path_buf();
    };
    let text: Vec<u16> = path.as_os_str().encode_wide().collect();
    let normal = match prefix.kind() {
        Prefix::VerbatimDisk(_) => text[4..].to_vec(),
        Prefix::VerbatimUNC(_, _) => [vec![92, 92], text[8..].to_vec()].concat(),
        _ => return path.to_path_buf(),
    };
    OsString::from_wide(&normal).into()
}
