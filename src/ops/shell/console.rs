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
    spawn_with_startup(command, current_dir, &startup_info())
}

fn startup_info() -> STARTUPINFOW {
    // No STARTF_USESTDHANDLES: Windows must create console handles instead of
    // inheriting FFM's raw input, output pipes, or alternate screen buffer.
    STARTUPINFOW {
        cb: std::mem::size_of::<STARTUPINFOW>() as u32,
        ..STARTUPINFOW::default()
    }
}

fn spawn_with_startup(
    command: &ShellCommand,
    current_dir: &Path,
    startup: &STARTUPINFOW,
) -> io::Result<OwnedHandle> {
    let shell = std::env::var_os("COMSPEC").unwrap_or_else(|| "cmd.exe".into());
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

#[allow(unsafe_code)]
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
    // SAFETY: successful CreateProcessW transfers ownership of these handles.
    unsafe {
        drop(OwnedHandle::from_raw_handle(process.hThread));
        Ok(OwnedHandle::from_raw_handle(process.hProcess))
    }
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
