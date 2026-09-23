//! Purpose: Pin what the status bar and the transcript say about how a `:`
//! shell command ended: its drawable output, a cancel, a timeout, a missing
//! folder.

use super::tests::make_app;
use super::*;
use tempfile::TempDir;

fn transcript(app: &App) -> Vec<String> {
    app.console
        .visible(usize::MAX)
        .into_iter()
        .map(|line| line.text().to_owned())
        .collect()
}

/// `ESC ( B` is what `tput sgr0` and ncurses programs print; its final byte
/// used to reach both the status bar and the transcript as a stray `B`.
#[cfg(unix)]
#[test]
fn charset_escapes_leave_no_stray_characters_in_the_status_or_the_transcript() {
    let temp = TempDir::new().expect("temp dir");
    let mut app = make_app(&temp);

    execute_command(&mut app, r"printf '\033(Bhello\n'").expect("shell command");
    assert_eq!(app.status_text(), "hello");
    assert_eq!(transcript(&app).last().map(String::as_str), Some("hello"));

    // Output with nothing drawable in it reads like no output at all.
    let reset = r"printf '\033(B\033[m'";
    execute_command(&mut app, reset).expect("shell command");
    assert_eq!(app.status_text(), format!("OK: {reset}"));
}

/// Esc / Ctrl+C is the user's own choice: reported as a cancel, and not as an
/// error that suggests the command never started.
#[test]
fn a_cancelled_command_says_so_in_the_status_and_the_transcript() {
    let temp = TempDir::new().expect("temp dir");
    let mut app = make_app(&temp);

    record_cancel(&mut app, "sleep 30; echo done");

    assert_eq!(app.status_text(), "cancelled: sleep 30; echo done");
    assert_eq!(
        transcript(&app).last().map(String::as_str),
        Some("cancelled: sleep 30; echo done")
    );
}

#[cfg(unix)]
#[test]
fn a_command_past_the_time_limit_is_reported_as_stopped_not_as_unrunnable() {
    let temp = TempDir::new().expect("temp dir");
    let mut app = make_app(&temp);
    app.settings
        .set(crate::menu::SettingsPage::PluginManager, "1")
        .expect("one second");

    let error = execute_command(&mut app, "sleep 5").expect_err("time limit");

    let expected = "stopped 'sleep 5': timed out after 1s (Esc stops it sooner)";
    assert_eq!(error.to_string(), expected);
    assert_eq!(transcript(&app).last().map(String::as_str), Some(expected));
}

/// The panel folder was deleted behind FFM's back. The OS reports that as
/// "No such file or directory", which reads as if `echo` were missing; the
/// message names the folder instead.
#[test]
fn a_command_run_from_a_deleted_folder_names_the_folder() {
    let temp = TempDir::new().expect("temp dir");
    let mut app = make_app(&temp);
    let gone = temp.path().join("gone");
    app.get_active_panel().path = gone.clone();

    let error = execute_command(&mut app, "echo hi").expect_err("nowhere to run");

    let expected = format!(
        "cannot run 'echo hi': folder {} no longer exists",
        gone.display()
    );
    assert_eq!(error.to_string(), expected);
    assert_eq!(transcript(&app).last(), Some(&expected));
}
