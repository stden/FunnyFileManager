//! Command-line dispatch: parses and executes typed commands.

use crate::models::Lifecycle;
use std::{io, path::Path};

use crate::command::{Command, ShellCommand};
use crate::models::{App, BookmarkSlot, FileName, Overlay, SortMode};
use crate::status::Status;
use crate::utils::refresh_panels;
use crate::windows::{open_windows_performance_checker, open_windows_system_messages};

use super::shell::run_shell_command_with_timeout;
use super::viewer::open_mcp_status;

fn cmd_cd_path(app: &mut App, requested: &Path) -> io::Result<()> {
    // Expand `~` before the absolute-path test: `~/x` is a *relative* path, so
    // it would otherwise be joined onto the panel directory and never found.
    let requested = crate::command::expand_tilde(requested);
    // When HOME/USERPROFILE is unset, expand_tilde leaves the path starting
    // with `~`. Joining that onto the panel would open a literal `./~`
    // subdirectory if one exists — not the user's home. Refuse instead.
    if requested.as_os_str().as_encoded_bytes().first().copied() == Some(b'~') {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "home directory is not set (HOME / USERPROFILE)",
        ));
    }
    let new_path = if requested.is_absolute() {
        requested
    } else {
        app.get_active_panel().path.join(requested)
    }
    .canonicalize()?;

    if !new_path.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("not a directory: {}", new_path.display()),
        ));
    }

    // Through `navigate_to`: it drops the name-keyed marks that belonged to the
    // old directory and rolls back if the listing fails. `is_dir` above is not
    // enough — a directory we may traverse but not read (`cd /root` as a normal
    // user) passes every check here and only fails at `read_dir`.
    app.get_active_panel().navigate_to(new_path)?;
    let opened = app.get_active_panel().path.clone();
    app.set_status(crate::status::Status::Opened(opened));
    Ok(())
}

/// `:sort <mode>` sets the mode outright and leaves the order alone: a typed
/// command is not a key pressed twice.
pub(super) fn cmd_sort_mode(app: &mut App, sort_mode: SortMode) -> io::Result<()> {
    app.get_active_panel().sort_mode = sort_mode;
    super::sortmenu::relist(app)
}

fn cmd_bookmark_set(app: &mut App, slot: BookmarkSlot) -> io::Result<()> {
    // No bounds check: `BookmarkSlot` proves the index and `Bookmarks` has one
    // entry per slot, so the store cannot be missing the one being written.
    let path = app.get_active_panel().path.clone();
    app.set_status(crate::status::Status::BookmarkSet {
        slot,
        path: path.clone(),
    });
    app.bookmarks.set(slot, path);
    Ok(())
}

fn cmd_bookmark_go(app: &mut App, slot: BookmarkSlot) -> io::Result<()> {
    let Some(path) = app.bookmarks.get(slot).map(Path::to_path_buf) else {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("bookmark {slot} is empty"),
        ));
    };
    // Through `navigate_to`: a bookmark is only a remembered path, and the
    // directory may well be gone by now. Nothing else here checks that it still
    // exists, so the rollback is the only thing keeping the panel off a path it
    // cannot list.
    app.get_active_panel().navigate_to(path)?;
    let path = app.get_active_panel().path.clone();
    app.set_status(crate::status::Status::OpenedBookmark { slot, path });
    Ok(())
}

fn cmd_mark_all(app: &mut App) {
    let names: Vec<FileName> = app
        .get_active_panel()
        .entries
        .iter()
        .filter(|entry| entry.name != "..")
        .map(|entry| FileName::new(entry.name.clone()))
        .collect();
    let panel = app.get_active_panel();
    panel.marked.extend(names);
    let count = panel.marked.len();
    app.set_status(crate::status::Status::Marked { count });
}

pub fn execute_command(app: &mut App, command: &str) -> io::Result<()> {
    execute_command_with_console(app, command, super::shell::spawn_console)
}

fn execute_command_with_console(
    app: &mut App,
    command: &str,
    spawn_console: impl FnOnce(&ShellCommand, &Path) -> io::Result<()>,
) -> io::Result<()> {
    let parsed = Command::parse(command);
    // Echo first, so a command that opens an overlay or quits still leaves its
    // line in the transcript. Built-ins are echoed too: the console is a record
    // of the session, and a `cd` missing from it makes the surrounding shell
    // output impossible to place.
    let cwd = app.get_active_panel().path.clone();
    app.console.record_command(&cwd, command);

    match parsed {
        Command::Cd(path) => return cmd_cd_path(app, &path),
        Command::Sort(mode) => return cmd_sort_mode(app, mode),
        Command::BookmarkSet(slot) => return cmd_bookmark_set(app, slot),
        Command::BookmarkGo(slot) => return cmd_bookmark_go(app, slot),
        Command::WinMsg => return open_windows_system_messages(app),
        Command::WinPerf => return open_windows_performance_checker(app),
        Command::DiskCleanup => return crate::ops::disk_cleanup::open(app),
        Command::Mcp => return open_mcp_status(app),
        Command::Shell(shell) => return cmd_shell(app, &shell),
        Command::NewConsole(shell) => return cmd_new_console(app, &shell, spawn_console),
        Command::Invalid { usage, .. } => {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, usage))
        }
        Command::Empty => return Err(io::Error::new(io::ErrorKind::InvalidInput, "empty command")),
        Command::Pwd => cmd_pwd(app),
        Command::MarkAll => cmd_mark_all(app),
        Command::UnmarkAll => cmd_unmark_all(app),
        Command::Help => app.set_overlay(Overlay::Help),
        Command::Stats => super::performance::open(app),
        Command::Wsl => super::wsl::open_manager(app),
        Command::Quit => app.lifecycle = Lifecycle::Quitting,
    }
    Ok(())
}

fn cmd_pwd(app: &mut App) {
    let path = app.get_active_panel().path.clone();
    app.set_status(Status::Path(path));
}

fn cmd_unmark_all(app: &mut App) {
    app.get_active_panel().clear_marks();
    app.set_status(Status::UnmarkedAll);
}

fn cmd_new_console(
    app: &mut App,
    command: &ShellCommand,
    spawn: impl FnOnce(&ShellCommand, &Path) -> io::Result<()>,
) -> io::Result<()> {
    spawn(command, &app.get_active_panel_ref().path)
        .map_err(|error| command_launch_error(app, command.as_str(), &error))?;
    let status = Status::ConsoleStarted(command.clone());
    app.console.record_output(&status.to_string());
    app.set_status(status);
    Ok(())
}

fn command_launch_error(app: &mut App, command: &str, error: &io::Error) -> io::Error {
    let message = format!("cannot run '{command}': {error}");
    app.console.record_error(&message);
    io::Error::other(message)
}

/// Run `command` in the active panel's directory and report what it printed.
///
/// Everything the command wrote goes to the console transcript behind the
/// panels (`Ctrl+O` reveals it), the way Far Manager prints to the console
/// screen. The status bar keeps its one-line summary, so the common case —
/// glance at the result, keep working — costs no keystrokes at all.
fn cmd_shell(app: &mut App, command: &ShellCommand) -> io::Result<()> {
    // No emptiness check: `ShellCommand` cannot be constructed from blank text,
    // so "run nothing" is not a state this function can be called in.
    let command = command.as_str();
    let panel_path = app.get_active_panel().path.clone();
    let timeout = std::time::Duration::from_secs(
        app.settings
            .number(crate::menu::SettingsPage::PluginManager) as u64,
    );
    let output =
        run_shell_command_with_timeout(command, &panel_path, timeout).map_err(|error| {
            // A shell that would not start is still part of the transcript: the
            // echoed command with nothing under it reads as "it worked silently".
            command_launch_error(app, command, &error)
        })?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    app.console.record_output(&stdout);
    app.console.record_error(&stderr);
    app.set_status(shell_summary(&stdout, &stderr, output.status, command));

    // The command may have created or removed files; a refresh failure is not
    // the command's failure, so it is deliberately not propagated.
    let _ = refresh_panels(app);
    Ok(())
}

/// The one line the status bar shows for a finished command.
fn shell_summary(
    stdout: &str,
    stderr: &str,
    status: std::process::ExitStatus,
    command: &str,
) -> Status {
    // Through the same sanitiser the transcript uses: a `ls --color` summary
    // would otherwise read `[31mred[0m` in the status bar while the console
    // two keystrokes away shows `red`.
    if !stdout.trim().is_empty() {
        return Status::CommandOutput(crate::console::printable(
            stdout.lines().last().unwrap_or(""),
        ));
    }
    if !stderr.trim().is_empty() {
        return Status::CommandOutput(format!(
            "Err: {}",
            crate::console::printable(stderr.lines().next().unwrap_or(""))
        ));
    }
    if status.success() {
        return Status::CommandOutput(format!("OK: {command}"));
    }
    Status::CommandOutput(format!("Exit {status}"))
}

#[cfg(test)]
mod tests_navigation;

#[cfg(test)]
mod tests;

#[cfg(all(test, windows))]
mod tests_console;

#[cfg(test)]
mod tests_console_effect;
