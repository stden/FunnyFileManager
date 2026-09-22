//! Unit tests for [`super`], moved out of the module so their
//! assertion-failure arms are not measured as unreached production lines.

use super::*;
use crate::models::{App, Overlay, Panel};
use std::fs::{self, File};
use tempfile::TempDir;

pub(super) fn make_app(temp: &TempDir) -> App {
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
