//! Purpose: Open panel documents with the host OS association without waiting for the application.

use std::{io, path::Path};

use crate::{models::App, status::Status};

mod headless;

pub fn selected(app: &mut App) -> io::Result<()> {
    let Some(path) = app.get_active_panel().selected_path() else {
        return Ok(());
    };
    let path = std::path::absolute(path)?;
    crate::diagnostics::log_action("SystemOpen: requested");
    let result = if app.live.terminal_mode == crate::models::TerminalMode::Headless {
        headless::open(&path)
    } else {
        open(&path)
    };
    match result {
        Ok(()) => app.set_status(Status::Cleared),
        Err(error) => app.set_status(Status::CannotRun {
            program: path,
            error: error.to_string(),
        }),
    }
    Ok(())
}

#[cfg(not(test))]
fn open(path: &Path) -> io::Result<()> {
    host_open(path)
}

#[cfg(all(windows, not(test)))]
fn host_open(path: &Path) -> io::Result<()> {
    crate::platform::open_associated(path)
}

#[cfg(all(not(windows), not(test)))]
fn host_open(path: &Path) -> io::Result<()> {
    use std::process::{Command, Stdio};
    let program = if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    let status = Command::new(program)
        .arg(path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other("The system could not open this file"))
    }
}

#[cfg(test)]
std::thread_local! {
    static OPENED: std::cell::RefCell<Option<std::path::PathBuf>> = const {
        std::cell::RefCell::new(None)
    };
}

#[cfg(test)]
fn open(path: &Path) -> io::Result<()> {
    // Keep unit tests from opening desktop applications on the developer's machine.
    OPENED.with_borrow_mut(|opened| *opened = Some(path.to_owned()));
    Ok(())
}

#[cfg(test)]
pub fn take_opened() -> Option<std::path::PathBuf> {
    OPENED.with_borrow_mut(Option::take)
}
