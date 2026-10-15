//! Command-line dispatch: parses and executes typed commands.

use crate::models::Lifecycle;
use std::{io, path::Path};

use crate::command::{Command, ShellCommand};
use crate::models::{App, BookmarkSlot, Overlay, SortMode};
use crate::status::Status;
use crate::windows::{open_windows_performance_checker, open_windows_system_messages};

use super::viewer::open_mcp_status;

mod shell_commands;

use shell_commands::{cmd_new_console, cmd_shell};

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
    let typed = if requested.is_absolute() {
        requested
    } else {
        app.get_active_panel().path.join(requested)
    };
    let new_path = typed
        .canonicalize()
        .map_err(|error| crate::utils::FailedAt::wrap(&typed, error))?;
    // `canonicalize` prepends `\\?\` on Windows. Strip it so the panel path
    // stays in the ordinary form used everywhere else.
    let new_path = crate::path_display::normalize(new_path);

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
    app.set_status(Status::Opened(opened));
    Ok(())
}

/// `:sort <mode>` sets the mode outright and leaves the order alone: a typed
/// command is not a key pressed twice.
pub(super) fn cmd_sort_mode(app: &mut App, sort_mode: SortMode) -> io::Result<()> {
    app.get_active_panel().sort_mode = sort_mode;
    super::sortmenu::relist(app)
}

fn cmd_bookmark_set(app: &mut App, slot: BookmarkSlot) {
    // No bounds check: `BookmarkSlot` proves the index and `Bookmarks` has one
    // entry per slot, so the store cannot be missing the one being written.
    let path = app.get_active_panel().path.clone();
    app.set_status(Status::BookmarkSet {
        slot,
        path: path.clone(),
    });
    // The slot is set for this run either way; a file that cannot be written
    // goes to the log, as the other stores' failures do.
    if let Err(error) = app.bookmarks.remember(slot, path) {
        crate::diagnostics::log_error(&format!("saving bookmarks: {error}"));
    }
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
    app.set_status(Status::OpenedBookmark { slot, path });
    Ok(())
}

fn cmd_mark_all(app: &mut App) {
    let panel = app.get_active_panel();
    panel.mark_all();
    let count = panel.marked.len();
    app.set_status(Status::Marked { count });
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
        Command::BookmarkGo(slot) => return cmd_bookmark_go(app, slot),
        Command::WinMsg => return open_windows_system_messages(app),
        Command::WinPerf => return open_windows_performance_checker(app),
        Command::DiskCleanup => return crate::ops::disk_cleanup::open(app),
        Command::Mcp => return open_mcp_status(app),
        Command::Sessions(command) => return super::sessions::execute(app, command),
        Command::Shell(shell) => return cmd_shell(app, &shell),
        Command::NewConsole(shell) => return cmd_new_console(app, &shell, spawn_console),
        Command::Invalid { usage, .. } => {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, usage))
        }
        Command::Empty => return Err(io::Error::new(io::ErrorKind::InvalidInput, "empty command")),
        Command::BookmarkSet(slot) => cmd_bookmark_set(app, slot),
        Command::Pwd => cmd_pwd(app),
        Command::MarkAll => cmd_mark_all(app),
        Command::UnmarkAll => cmd_unmark_all(app),
        Command::Help => app.set_overlay(Overlay::Help),
        Command::Stats => super::performance::open(app),
        Command::Lsp => super::lsp_editor::open(app),
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

#[cfg(test)]
mod tests_navigation;

#[cfg(test)]
mod tests;

#[cfg(all(test, windows))]
mod tests_console;

#[cfg(test)]
mod tests_console_effect;

#[cfg(test)]
mod tests_shell_outcomes;
