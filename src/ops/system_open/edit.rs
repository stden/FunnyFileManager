//! Purpose: Open image files with the host OS image editor without waiting for it.

use std::{io, path::Path};

use crate::models::{App, TerminalMode};

#[cfg(any(windows, test))]
const ERROR_NO_ASSOCIATION: i32 = 1155;

pub fn edit_path(app: &App, path: &Path) -> io::Result<()> {
    let path = std::path::absolute(path)?;
    crate::diagnostics::log_action("SystemOpen: image edit requested");
    if app.live.terminal_mode == TerminalMode::Headless {
        super::headless::edit(&path)
    } else {
        edit(&path)
    }
}

#[cfg(any(windows, test))]
fn edit_with_fallback(
    path: &Path,
    primary: fn(&Path) -> io::Result<()>,
    fallback: fn(&Path) -> io::Result<()>,
) -> io::Result<()> {
    match primary(path) {
        Ok(()) => Ok(()),
        Err(error) if error.raw_os_error() == Some(ERROR_NO_ASSOCIATION) => fallback(path),
        Err(error) => Err(error),
    }
}

#[cfg(not(test))]
fn edit(path: &Path) -> io::Result<()> {
    host_edit(path)
}

#[cfg(all(windows, not(test)))]
fn host_edit(path: &Path) -> io::Result<()> {
    edit_with_fallback(
        path,
        crate::platform::edit_associated,
        crate::platform::open_associated,
    )
}

#[cfg(not(windows))]
pub(super) fn unix_command(path: &Path) -> std::process::Command {
    use std::process::{Command, Stdio};

    let configured = std::env::var("IMAGE_EDITOR").unwrap_or_default();
    let trimmed = configured.trim();
    let mut parts = trimmed.split_whitespace();
    let mut command = if !trimmed.is_empty() && Path::new(trimmed).is_file() {
        Command::new(trimmed)
    } else if let Some(program) = parts.next() {
        let mut cmd = Command::new(program);
        cmd.args(parts);
        cmd
    } else {
        #[cfg(target_os = "macos")]
        const OPENER: &str = "open";
        #[cfg(not(target_os = "macos"))]
        const OPENER: &str = "xdg-open";
        Command::new(OPENER)
    };
    command
        .arg(path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    command
}

#[cfg(all(not(windows), not(test)))]
fn host_edit(path: &Path) -> io::Result<()> {
    let status = unix_command(path).status()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(
            "The system image editor could not open this file",
        ))
    }
}

#[cfg(test)]
std::thread_local! {
    static EDITED: std::cell::RefCell<Option<std::path::PathBuf>> = const {
        std::cell::RefCell::new(None)
    };
    static FORCE_ERROR: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

#[cfg(test)]
fn edit(path: &Path) -> io::Result<()> {
    if FORCE_ERROR.replace(false) {
        return Err(io::Error::other("forced image editor failure"));
    }
    EDITED.with_borrow_mut(|edited| *edited = Some(path.to_owned()));
    Ok(())
}

#[cfg(test)]
pub fn take_edited() -> Option<std::path::PathBuf> {
    EDITED.with_borrow_mut(Option::take)
}

#[cfg(test)]
pub fn fail_next_edit() {
    FORCE_ERROR.set(true);
}

#[cfg(test)]
mod tests;
