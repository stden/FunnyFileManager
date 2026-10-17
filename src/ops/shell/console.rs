//! Windows console launch with fresh standard handles, independent of the TUI.

use std::ffi::OsStr;
use std::os::windows::{
    ffi::OsStrExt,
    io::{FromRawHandle, OwnedHandle},
};
use std::{io, path::Path, ptr};

use windows_sys::Win32::System::Threading::{
    CreateProcessW, CREATE_NEW_CONSOLE, PROCESS_INFORMATION, STARTUPINFOW,
};

use crate::command::ShellCommand;

pub(super) fn spawn(command: &ShellCommand, current_dir: &Path) -> io::Result<OwnedHandle> {
    spawn_with_startup(command, current_dir, &startup_info()?)
}

fn startup_info() -> io::Result<STARTUPINFOW> {
    // No STARTF_USESTDHANDLES: Windows must create console handles instead of
    // inheriting FFM's raw input, output pipes, or alternate screen buffer.
    Ok(STARTUPINFOW {
        cb: u32::try_from(size_of::<STARTUPINFOW>()).map_err(io::Error::other)?,
        ..STARTUPINFOW::default()
    })
}

/// Build the command-line string passed to `CreateProcessW` with inner
/// double-quotes doubled — documents the VULN-05 escaping pattern.
///
/// Not used in the live spawn path: quote-doubling changes how quoted
/// arguments (`"two words"`) reach launched programs and breaks existing
/// usage (VULN-05 is low severity, user-controlled input only).  The live
/// path keeps the original format so that quoted word-grouping works.
#[cfg(test)]
fn cmd_line(command: &ShellCommand) -> String {
    let escaped = command.as_str().replace('"', "\"\"");
    format!("cmd.exe /D /S /K \"{escaped}\"")
}

fn spawn_with_startup(
    command: &ShellCommand,
    current_dir: &Path,
    startup: &STARTUPINFOW,
) -> io::Result<OwnedHandle> {
    let shell = std::env::var_os("COMSPEC")
        .filter(|s| Path::new(s).is_absolute())
        .unwrap_or_else(|| {
            std::env::var_os("SystemRoot")
                .map_or_else(
                    || std::path::PathBuf::from("C:\\Windows\\System32\\cmd.exe"),
                    |r| std::path::PathBuf::from(r).join("System32\\cmd.exe"),
                )
                .into_os_string()
        });
    let shell = wide(&shell)?;
    // /S strips only the outer quotes. Keep the user's argument quoting intact;
    // /K leaves errors (including a missing codex installation) visible.
    let mut line = wide(OsStr::new(&format!(
        "cmd.exe /D /S /K \"{}\"",
        command.as_str()
    )))?;
    let directory = crate::windows::console_directory(&std::path::absolute(current_dir)?);
    let directory = wide(directory.as_os_str())?;
    create_process(&shell, &mut line, &directory, startup)
}

#[allow(unsafe_code, reason = "CreateProcessW has no safe wrapper in std")]
fn create_process(
    shell: &[u16],
    line: &mut [u16],
    directory: &[u16],
    startup: &STARTUPINFOW,
) -> io::Result<OwnedHandle> {
    let mut process = PROCESS_INFORMATION::default();
    // SAFETY: all strings are NUL-terminated and live throughout the call;
    // line is writable, and both Windows structures are initialized.
    let created = unsafe {
        CreateProcessW(
            shell.as_ptr(),
            line.as_mut_ptr(),
            ptr::null(),
            ptr::null(),
            0,
            CREATE_NEW_CONSOLE,
            ptr::null(),
            directory.as_ptr(),
            startup,
            &raw mut process,
        )
    };
    if created == 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: a successful CreateProcessW hands the thread handle over to us,
    // and this is the only place that takes ownership of it.
    let thread = unsafe { OwnedHandle::from_raw_handle(process.hThread) };
    drop(thread);
    // SAFETY: the same call hands over the process handle, owned only here.
    let child = unsafe { OwnedHandle::from_raw_handle(process.hProcess) };
    Ok(child)
}

fn wide(value: &OsStr) -> io::Result<Vec<u16>> {
    let mut encoded: Vec<u16> = value.encode_wide().collect();
    if encoded.contains(&0) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "console command or path contains a NUL character",
        ));
    }
    encoded.push(0);
    Ok(encoded)
}

#[cfg(test)]
mod tests;
