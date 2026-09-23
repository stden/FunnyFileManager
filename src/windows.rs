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

/// Own a child's process tree: closing the returned handle kills every
/// process in it, so a crashed or unresponsive LSP server, or a CLI stopped
/// at its time limit, cannot orphan children.
#[cfg(windows)]
#[allow(
    unsafe_code,
    reason = "std has no job objects, the only way to end a process tree with its handle"
)]
pub(crate) fn process_tree_job(
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
    configure_kill_on_close(raw)?;
    // SAFETY: both handles remain live throughout the call; the child is owned by the caller.
    if unsafe { AssignProcessToJobObject(raw, child.as_raw_handle()) } == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(job)
}

#[cfg(windows)]
#[allow(
    unsafe_code,
    reason = "std has no job objects, the only way to end a process tree with its handle"
)]
fn configure_kill_on_close(handle: windows_sys::Win32::Foundation::HANDLE) -> io::Result<()> {
    use windows_sys::Win32::System::JobObjects::{
        JobObjectExtendedLimitInformation, SetInformationJobObject,
        JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    };
    let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
    limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
    let size = u32::try_from(size_of_val(&limits)).map_err(io::Error::other)?;
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
        lines.push("<no data>".to_owned());
    }

    lines
}

#[cfg(any(test, windows))]
fn open_report_viewer(app: &mut App, title: &str, report: &str) {
    app.set_overlay(crate::models::Overlay::Viewer(Viewer::report(
        title,
        report_lines(report),
    )));
    app.set_status(crate::status::Status::OpenedView(title.to_owned()));
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

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();

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
            "Command exited with failure status".to_owned(),
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
    // `\\?\C:\…` drops its `\\?\`; `\\?\UNC\server\…` trades `\\?\UNC\` for `\\`.
    let normal = match prefix.kind() {
        Prefix::VerbatimDisk(_) => text.get(4..).map(<[u16]>::to_vec),
        Prefix::VerbatimUNC(_, _) => text
            .get(8..)
            .map(|share| [[92, 92].as_slice(), share].concat()),
        _ => None,
    };
    normal.map_or_else(
        || path.to_path_buf(),
        |normal| OsString::from_wide(&normal).into(),
    )
}
