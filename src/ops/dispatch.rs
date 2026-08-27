//! Command-line dispatch: parses and executes typed commands.

use crate::models::Lifecycle;
use std::{io, path::Path};

use crate::command::{Command, ShellCommand};
use crate::models::{App, BookmarkSlot, FileName, Overlay, SortMode};
use crate::status::Status;
use crate::utils::refresh_panels;
use crate::windows::{open_windows_performance_checker, open_windows_system_messages};

use super::shell::run_shell_command;
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

fn cmd_sort_mode(app: &mut App, sort_mode: SortMode) -> io::Result<()> {
    let panel = app.get_active_panel();
    panel.sort_mode = sort_mode;
    panel.load_entries()?;
    panel.ensure_selection();
    let label = panel.sort_mode.label().to_string();
    app.set_status(crate::status::Status::Sort(label));
    Ok(())
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
    let parsed = Command::parse(command);
    // Echo first, so a command that opens an overlay or quits still leaves its
    // line in the transcript. Built-ins are echoed too: the console is a record
    // of the session, and a `cd` missing from it makes the surrounding shell
    // output impossible to place.
    let cwd = app.get_active_panel().path.clone();
    app.console.record_command(&cwd, command);

    match parsed {
        Command::Cd(path_arg) => cmd_cd_path(app, &path_arg),
        Command::Pwd => {
            let path = app.get_active_panel().path.clone();
            app.set_status(Status::Path(path));
            Ok(())
        }
        Command::Sort(mode) => cmd_sort_mode(app, mode),
        Command::BookmarkSet(slot) => cmd_bookmark_set(app, slot),
        Command::BookmarkGo(slot) => cmd_bookmark_go(app, slot),
        Command::MarkAll => {
            cmd_mark_all(app);
            Ok(())
        }
        Command::UnmarkAll => {
            app.get_active_panel().clear_marks();
            app.set_status(crate::status::Status::UnmarkedAll);
            Ok(())
        }
        Command::Help => {
            app.set_overlay(Overlay::Help);
            Ok(())
        }
        Command::WinMsg => open_windows_system_messages(app),
        Command::WinPerf => open_windows_performance_checker(app),
        Command::Mcp => open_mcp_status(app),
        Command::Quit => {
            app.lifecycle = Lifecycle::Quitting;
            Ok(())
        }
        Command::Invalid { usage, .. } => Err(io::Error::new(io::ErrorKind::InvalidInput, usage)),
        Command::Shell(shell) => cmd_shell(app, &shell),
        Command::Empty => Err(io::Error::new(io::ErrorKind::InvalidInput, "empty command")),
    }
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
    let output = run_shell_command(command, &panel_path).map_err(|error| {
        // A shell that would not start is still part of the transcript: the
        // echoed command with nothing under it reads as "it worked silently".
        let message = format!("cannot run '{command}': {error}");
        app.console.record_error(&message);
        io::Error::other(message)
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
mod tests {
    use super::*;
    use crate::models::{App, Overlay, Panel};
    use std::fs::{self, File};
    use tempfile::TempDir;

    fn make_app(temp: &TempDir) -> App {
        let mut app = App::new().expect("app");
        app.left_panel = Panel::new(temp.path().to_path_buf()).expect("panel");
        app.right_panel = Panel::new(temp.path().to_path_buf()).expect("panel");
        app
    }

    // ─── cd ───────────────────────────────────────────────────────────────

    #[test]
    fn test_cd_absolute_path() {
        let temp = TempDir::new().expect("tempdir");
        let subdir = temp.path().join("sub");
        fs::create_dir(&subdir).expect("mkdir");

        let mut app = make_app(&temp);
        execute_command(&mut app, &format!("cd {}", subdir.display())).expect("cd");
        assert_eq!(app.get_active_panel().path.file_name().unwrap(), "sub");
        assert!(app.status_text().contains("Opened"));
    }

    #[test]
    fn test_cd_relative_path() {
        let temp = TempDir::new().expect("tempdir");
        let subdir = temp.path().join("child");
        fs::create_dir(&subdir).expect("mkdir");

        let mut app = make_app(&temp);
        execute_command(&mut app, "cd child").expect("cd");
        assert_eq!(app.get_active_panel().path.file_name().unwrap(), "child");
    }

    #[test]
    fn test_cd_nonexistent_errors() {
        let temp = TempDir::new().expect("tempdir");
        let mut app = make_app(&temp);
        let result = execute_command(&mut app, "cd nonexistent_xyz");
        assert!(result.is_err());
    }

    #[test]
    fn test_cd_to_file_errors() {
        let temp = TempDir::new().expect("tempdir");
        File::create(temp.path().join("afile.txt")).expect("create");
        let mut app = make_app(&temp);
        let result = execute_command(
            &mut app,
            &format!("cd {}", temp.path().join("afile.txt").display()),
        );
        assert!(result.is_err());
    }

    // ─── pwd ──────────────────────────────────────────────────────────────

    #[test]
    fn test_pwd_sets_status() {
        let temp = TempDir::new().expect("tempdir");
        let mut app = make_app(&temp);
        execute_command(&mut app, "pwd").expect("pwd");
        assert!(!app.status_text().is_empty());
    }

    // ─── sort ─────────────────────────────────────────────────────────────

    #[test]
    fn test_sort_name() {
        let temp = TempDir::new().expect("tempdir");
        let mut app = make_app(&temp);
        execute_command(&mut app, "sort name").expect("sort");
        assert_eq!(app.get_active_panel().sort_mode, SortMode::Name);
        assert!(app.status_text().contains("Sort"));
    }

    #[test]
    fn test_sort_extension() {
        let temp = TempDir::new().expect("tempdir");
        let mut app = make_app(&temp);
        execute_command(&mut app, "sort ext").expect("sort");
        assert_eq!(app.get_active_panel().sort_mode, SortMode::Extension);
    }

    #[test]
    fn test_sort_size() {
        let temp = TempDir::new().expect("tempdir");
        let mut app = make_app(&temp);
        execute_command(&mut app, "sort size").expect("sort");
        assert_eq!(app.get_active_panel().sort_mode, SortMode::Size);
    }

    // ─── bookmarks ───────────────────────────────────────────────────────

    #[test]
    fn test_bookmark_set_and_go() {
        let temp = TempDir::new().expect("tempdir");
        let mut app = make_app(&temp);
        execute_command(&mut app, "bookmark set 0").expect("set");
        assert!(app
            .bookmarks
            .get(BookmarkSlot::from_usize(0).expect("slot"))
            .is_some());
        assert!(app.status_text().contains("Bookmark 0 set"));

        execute_command(&mut app, "bookmark go 0").expect("go");
        assert!(app.status_text().contains("Opened bookmark 0"));
    }

    #[test]
    fn test_bookmark_go_empty_errors() {
        let temp = TempDir::new().expect("tempdir");
        let mut app = make_app(&temp);
        let result = execute_command(&mut app, "bookmark go 5");
        assert!(result.is_err());
    }

    // ─── mark / unmark ───────────────────────────────────────────────────

    #[test]
    fn test_mark_all_and_unmark() {
        let temp = TempDir::new().expect("tempdir");
        File::create(temp.path().join("a.txt")).expect("create");
        File::create(temp.path().join("b.txt")).expect("create");

        let mut app = make_app(&temp);
        app.left_panel.load_entries().expect("load");

        execute_command(&mut app, "mark all").expect("mark");
        assert!(!app.get_active_panel().marked.is_empty());
        assert!(app.status_text().contains("Marked"));

        execute_command(&mut app, "unmark").expect("unmark");
        assert!(app.get_active_panel().marked.is_empty());
        assert_eq!(app.status_text(), "Unmarked all");
    }

    // ─── help / quit ─────────────────────────────────────────────────────

    #[test]
    fn test_help_opens_overlay() {
        let temp = TempDir::new().expect("tempdir");
        let mut app = make_app(&temp);
        execute_command(&mut app, "help").expect("help");
        assert!(matches!(app.overlay, Overlay::Help));
    }

    #[test]
    fn test_quit_sets_flag() {
        let temp = TempDir::new().expect("tempdir");
        let mut app = make_app(&temp);
        execute_command(&mut app, "quit").expect("quit");
        assert!(!app.lifecycle.is_running());
    }

    #[test]
    fn test_exit_sets_flag() {
        let temp = TempDir::new().expect("tempdir");
        let mut app = make_app(&temp);
        execute_command(&mut app, "exit").expect("exit");
        assert!(!app.lifecycle.is_running());
    }

    // ─── invalid ─────────────────────────────────────────────────────────

    #[test]
    fn test_invalid_command_errors() {
        let temp = TempDir::new().expect("tempdir");
        let mut app = make_app(&temp);
        let result = execute_command(&mut app, "sort garbage");
        assert!(result.is_err());
    }

    // ─── shell ───────────────────────────────────────────────────────────

    #[test]
    fn test_shell_echo_stdout() {
        let temp = TempDir::new().expect("tempdir");
        let mut app = make_app(&temp);
        execute_command(&mut app, "echo world").expect("shell");
        assert!(app.status_text().contains("world"));
    }

    #[test]
    fn test_shell_empty_errors() {
        let temp = TempDir::new().expect("tempdir");
        let mut app = make_app(&temp);
        // Empty shell (just spaces) should fail
        let result = execute_command(&mut app, "   ");
        // The parser treats whitespace-only as Shell("") which is empty
        // Actually split_whitespace returns None for "   ", so it's Shell("")
        // Then the Shell arm checks is_empty and returns InvalidInput
        assert!(result.is_err());
    }

    #[test]
    fn test_shell_stderr_shows_err_prefix() {
        let temp = TempDir::new().expect("tempdir");
        let mut app = make_app(&temp);
        #[cfg(windows)]
        let cmd = "echo oops 1>&2";
        #[cfg(not(windows))]
        let cmd = "echo oops >&2";
        execute_command(&mut app, cmd).expect("shell");
        assert!(app.status_text().starts_with("Err:"));
        assert!(app.status_text().contains("oops"));
    }

    #[test]
    fn test_shell_success_no_output_shows_ok() {
        let temp = TempDir::new().expect("tempdir");
        let mut app = make_app(&temp);
        #[cfg(windows)]
        let cmd = "ver >nul 2>nul";
        #[cfg(not(windows))]
        let cmd = "true";
        execute_command(&mut app, cmd).expect("shell");
        assert!(
            app.status_text().starts_with("OK:"),
            "expected OK: prefix, got: {}",
            app.status_text()
        );
    }

    #[test]
    fn test_shell_failure_shows_exit() {
        let temp = TempDir::new().expect("tempdir");
        let mut app = make_app(&temp);
        #[cfg(windows)]
        let cmd = "cmd /c exit /b 1";
        #[cfg(not(windows))]
        let cmd = "false";
        execute_command(&mut app, cmd).expect("shell");
        assert!(
            app.status_text().starts_with("Exit"),
            "expected Exit prefix, got: {}",
            app.status_text()
        );
    }

    // ─── mcp ─────────────────────────────────────────────────────────────

    #[test]
    fn test_mcp_opens_viewer() {
        let temp = TempDir::new().expect("tempdir");
        let mut app = make_app(&temp);
        execute_command(&mut app, "mcp").expect("mcp");
        assert!(app.get_viewer().is_some());
        assert!(app.status_text().contains("MCP"));
    }

    /// Setting a bookmark cannot fail for lack of a slot to put it in.
    ///
    /// This used to be a bounds test: it emptied the bookmarks `Vec` and
    /// asserted both commands errored. `Bookmarks` has one entry per
    /// `BookmarkSlot` by construction, so that state no longer exists — what is
    /// left to check is that every legal slot round-trips, and that an *empty*
    /// slot is still the one thing `bookmark go` refuses.
    #[test]
    fn every_legal_slot_round_trips_and_only_an_empty_one_refuses() {
        let temp = TempDir::new().expect("tempdir");
        let mut app = make_app(&temp);

        for n in 0..=BookmarkSlot::MAX {
            let slot = BookmarkSlot::new(n).expect("legal slot");
            assert!(
                cmd_bookmark_go(&mut app, slot).is_err(),
                "slot {n} is empty, so going to it must fail"
            );
            cmd_bookmark_set(&mut app, slot).expect("set cannot fail");
            cmd_bookmark_go(&mut app, slot).expect("go after set");
        }
    }

    // ─── cd: spaces, quotes and `~` ───────────────────────────────────────

    #[test]
    fn test_cd_into_directory_with_spaces() {
        let temp = TempDir::new().expect("tempdir");
        fs::create_dir(temp.path().join("My Documents")).expect("mkdir");

        let mut app = make_app(&temp);
        execute_command(&mut app, "cd My Documents").expect("cd");
        assert_eq!(
            app.get_active_panel().path.file_name().unwrap(),
            "My Documents"
        );
    }

    #[test]
    fn test_cd_into_quoted_directory_with_spaces() {
        let temp = TempDir::new().expect("tempdir");
        fs::create_dir(temp.path().join("My Documents")).expect("mkdir");

        let mut app = make_app(&temp);
        execute_command(&mut app, "cd \"My Documents\"").expect("cd");
        assert_eq!(
            app.get_active_panel().path.file_name().unwrap(),
            "My Documents"
        );
    }

    /// The completed line the tab-completer produces must execute unchanged.
    #[test]
    fn test_completed_cd_line_executes() {
        let temp = TempDir::new().expect("tempdir");
        fs::create_dir(temp.path().join("My Documents")).expect("mkdir");

        let mut app = make_app(&temp);
        let completion = crate::complete::complete("cd My", temp.path()).expect("completion");
        execute_command(&mut app, &completion.line).expect("cd");
        assert_eq!(
            app.get_active_panel().path.file_name().unwrap(),
            "My Documents"
        );
    }

    /// `cd ~` used to fail: `~/x` is a relative path, so it was joined onto the
    /// panel directory. Asserted structurally — other tests swap $HOME
    /// concurrently, so the exact destination is not stable, only that it is a
    /// real directory outside the panel we started in.
    /// Put `previous` back into the environment, or clear the variable when
    /// there was nothing to restore.
    fn restore_env(name: &str, previous: Option<std::ffi::OsString>) {
        match previous {
            Some(value) => std::env::set_var(name, value),
            None => std::env::remove_var(name),
        }
    }

    #[test]
    fn restore_env_sets_and_clears() {
        let _lock = crate::test_support::env_lock();
        let previous = std::env::var_os("FFM_DISPATCH_RESTORE_PROBE");

        restore_env("FFM_DISPATCH_RESTORE_PROBE", Some("probe".into()));
        assert_eq!(
            std::env::var_os("FFM_DISPATCH_RESTORE_PROBE").as_deref(),
            Some("probe".as_ref())
        );
        restore_env("FFM_DISPATCH_RESTORE_PROBE", None);
        assert!(std::env::var_os("FFM_DISPATCH_RESTORE_PROBE").is_none());

        restore_env("FFM_DISPATCH_RESTORE_PROBE", previous);
    }

    #[test]
    fn test_cd_tilde_opens_home_not_a_subdirectory_of_cwd() {
        let _lock = crate::test_support::env_lock();
        // Point HOME at a temp directory instead of skipping when it is unset,
        // so the test is deterministic in every environment.
        let prev_home = std::env::var_os("HOME");
        let home = TempDir::new().expect("home tempdir");
        std::env::set_var("HOME", home.path());

        let temp = TempDir::new().expect("tempdir");
        let mut app = make_app(&temp);

        let result = execute_command(&mut app, "cd ~");
        let landed = app.get_active_panel().path.clone();
        restore_env("HOME", prev_home);

        result.expect("cd ~");
        assert!(
            !landed.starts_with(temp.path()),
            "~ must not be resolved relative to the panel directory"
        );
    }

    /// With neither `$HOME` nor `%USERPROFILE%`, `~` is left unexpanded. Joining
    /// it onto the panel as a relative name would open `./~` if that existed;
    /// the command must fail (or stay put) rather than invent a home directory.
    #[test]
    fn test_cd_tilde_without_home_does_not_invent_a_directory() {
        let temp = TempDir::new().expect("tempdir");
        // A real `~` subdirectory would make a wrong expansion look successful.
        fs::create_dir(temp.path().join("~")).expect("mkdir literal tilde");

        let _lock = crate::test_support::env_lock();
        let prev_home = std::env::var_os("HOME");
        let prev_profile = std::env::var_os("USERPROFILE");
        std::env::remove_var("HOME");
        std::env::remove_var("USERPROFILE");

        let mut app = make_app(&temp);
        let before = app.get_active_panel().path.clone();
        let result = execute_command(&mut app, "cd ~");

        // Restore before any assert so a failure cannot leak empty HOME.
        restore_env("HOME", prev_home);
        restore_env("USERPROFILE", prev_profile);

        assert!(
            result.is_err() || app.get_active_panel().path == before,
            "without a home variable, cd ~ must not land on a panel-local '~' dir"
        );
        assert_ne!(
            app.get_active_panel()
                .path
                .file_name()
                .and_then(|n| n.to_str()),
            Some("~"),
            "panel must not open the literal '~' subdirectory as home"
        );
    }

    // ─── console transcript ───────────────────────────────────────────────

    /// The transcript as plain text, oldest first.
    fn console_text(app: &App) -> Vec<String> {
        app.console
            .visible(usize::MAX)
            .into_iter()
            .map(|line| line.text().to_owned())
            .collect()
    }

    #[test]
    fn command_output_lands_in_the_console_behind_the_panels() {
        let temp = TempDir::new().expect("temp dir");
        let mut app = make_app(&temp);

        execute_command(&mut app, r"printf 'alpha\nbeta\n'").expect("shell command");

        let transcript = console_text(&app);
        assert_eq!(
            transcript[0],
            format!(r"{}> printf 'alpha\nbeta\n'", temp.path().display()),
            "the command is echoed behind its prompt, as Far prints it"
        );
        assert_eq!(&transcript[1..], ["alpha".to_string(), "beta".to_string()]);
        // No window pops up over the panels — that is the whole point.
        assert!(matches!(app.overlay, Overlay::None));
        // The status bar still summarises, so the common case costs no keys.
        assert_eq!(app.status_text(), "beta");
    }

    #[test]
    fn stderr_is_recorded_after_stdout_and_tagged_as_an_error() {
        let temp = TempDir::new().expect("temp dir");
        let mut app = make_app(&temp);

        // `cmd.exe` chains with `&`, not `;` — it passes `;` on as an argument —
        // and `>&2` there redirects the *whole* line, so the redirect has to lead
        // and sit tight against the word to avoid echoing the space before it.
        #[cfg(windows)]
        let cmd = "echo out&1>&2 echo err";
        #[cfg(not(windows))]
        let cmd = r"printf 'out\n'; printf 'err\n' >&2";

        execute_command(&mut app, cmd).expect("shell command");

        let lines = app.console.visible(usize::MAX);
        assert!(matches!(lines[1], crate::console::ConsoleLine::Output(_)));
        assert!(matches!(lines[2], crate::console::ConsoleLine::Error(_)));
        assert_eq!(lines[1].text(), "out");
        assert_eq!(lines[2].text(), "err");
    }

    #[test]
    fn built_in_commands_are_echoed_too() {
        // A `cd` missing from the transcript makes the shell output around it
        // impossible to place.
        let temp = TempDir::new().expect("temp dir");
        let mut app = make_app(&temp);

        execute_command(&mut app, "pwd").expect("built-in");

        assert_eq!(
            console_text(&app),
            [format!("{}> pwd", temp.path().display())]
        );
    }

    #[test]
    fn a_command_that_cannot_be_launched_leaves_its_failure_in_the_console() {
        let temp = TempDir::new().expect("temp dir");
        let mut app = make_app(&temp);
        // The panel directory is gone, so the shell cannot even be spawned in it.
        app.get_active_panel().path = temp.path().join("does-not-exist");

        let result = execute_command(&mut app, "true");

        assert!(result.is_err());
        let transcript = console_text(&app);
        assert!(
            transcript
                .last()
                .is_some_and(|line| line.contains("cannot run 'true'")),
            "an echoed command with nothing under it reads as a silent success: {transcript:?}"
        );
    }

    #[test]
    fn the_status_bar_agrees_with_the_transcript_about_colour_codes() {
        // Otherwise the summary reads `[31mred[0m` while the console two
        // keystrokes away shows `red`.
        let temp = TempDir::new().expect("temp dir");
        let mut app = make_app(&temp);

        execute_command(&mut app, r"printf '\033[31mred\033[0m\n'").expect("shell command");

        assert_eq!(app.status_text(), "red");
        assert_eq!(console_text(&app).last().map(String::as_str), Some("red"));
    }

    #[test]
    fn an_empty_shell_command_is_rejected() {
        let temp = TempDir::new().expect("temp dir");
        let mut app = make_app(&temp);
        assert!(execute_command(&mut app, "   ").is_err());
    }
}
