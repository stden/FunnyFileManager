//! Start the local Print Spooler when the user requests it.

use std::io;

#[cfg(not(windows))]
pub(super) fn start_print_spooler() -> io::Result<&'static str> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "The Print Spooler service requires Windows",
    ))
}

#[cfg(all(windows, test))]
thread_local! {
    static SPOOLER_OVERRIDE: std::cell::RefCell<Option<std::path::PathBuf>> =
        const { std::cell::RefCell::new(None) };
}

#[cfg(windows)]
fn spooler_program() -> io::Result<std::path::PathBuf> {
    #[cfg(test)]
    if let Some(program) = SPOOLER_OVERRIDE.with(|slot| slot.borrow().clone()) {
        return Ok(program);
    }
    Ok(crate::platform::system_directory()?.join("sc.exe"))
}

#[cfg(windows)]
pub(super) fn start_print_spooler() -> io::Result<&'static str> {
    let program = spooler_program()?;
    start_with(&program, std::process::Command::status)
}

#[cfg(windows)]
fn start_with(
    program: &std::path::Path,
    run: impl FnOnce(&mut std::process::Command) -> io::Result<std::process::ExitStatus>,
) -> io::Result<&'static str> {
    use std::os::windows::process::CommandExt;
    use std::process::{Command, Stdio};
    use windows_sys::Win32::System::Threading::CREATE_NO_WINDOW;

    let status = run(Command::new(program)
        .args(["start", "Spooler"])
        .creation_flags(CREATE_NO_WINDOW)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null()))?;
    interpret_status_code(status.code())
}

#[cfg(any(windows, test))]
fn interpret_status_code(code: Option<i32>) -> io::Result<&'static str> {
    const SERVICE_SUCCESS: i32 = 0;
    const SERVICE_ALREADY_RUNNING: i32 = 1056;
    match code {
        Some(SERVICE_SUCCESS) => Ok("Print Spooler start requested"),
        Some(SERVICE_ALREADY_RUNNING) => Ok("Print Spooler is already running"),
        Some(raw) => Err(io::Error::from_raw_os_error(raw)),
        None => Err(io::Error::other("sc.exe terminated without an exit code")),
    }
}

#[cfg(test)]
mod tests;
