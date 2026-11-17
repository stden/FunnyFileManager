//! Unit tests for the task list's confirmations: kill, send a signal and
//! renice. The forms and pages it opens are in `tests_dialogs_info`.

use crossterm::event::KeyCode;

use super::tests::{draw, lines, sketch};
use crate::models::ProcessSample;
use crate::prompt::Privilege;
use crate::test_support::task_list::{key_of, named, pid, TaskApp, OWN};

/// The sketch with nothing marked, so a dialog is about the cursor's
/// process, 4242 firefox.
pub(super) fn cursor_only() -> TaskApp {
    let mut app = sketch();
    app.list().unmark_all();
    app
}

/// F8's buttons: Kill only where there are signals to choose between.
#[cfg(unix)]
const TERMINATE_BUTTONS: &str = "{ Terminate } [ Kill ] [ Cancel ]";
#[cfg(not(unix))]
const TERMINATE_BUTTONS: &str = "{ Terminate } [ Cancel ]";

/// The screen of `app` at 80×24, row by row.
pub(super) fn screen(app: &mut TaskApp) -> Vec<String> {
    let terminal = draw(app, 80, 24);
    lines(terminal.backend().buffer())
}

/// Whether some row of `screen` contains `text`.
fn shows(screen: &[String], text: &str) -> bool {
    screen.iter().any(|row| row.contains(text))
}

#[track_caller]
pub(super) fn assert_shows(screen: &[String], texts: &[&str]) {
    for text in texts {
        assert!(
            shows(screen, text),
            "{text:?} missing from\n{}",
            screen.join("\n")
        );
    }
}

#[test]
fn the_terminate_confirmation_names_its_target_and_offers_kill() {
    let mut app = cursor_only();
    app.press(KeyCode::F(8));
    let drawn = screen(&mut app);
    assert_shows(
        &drawn,
        &[
            " Terminate ",
            "Do you wish to terminate the process?",
            "4242 firefox",
            TERMINATE_BUTTONS,
        ],
    );
    assert!(
        app.app.dialog_mouse.area.is_some(),
        "the dialog was tracked"
    );
}

#[test]
fn a_confirmation_over_many_counts_the_unnamed_and_lists_the_skipped() {
    let mut processes: Vec<ProcessSample> = (1..=8).map(|raw| named(100 + raw, "worker")).collect();
    processes.push(named(OWN, "ffm"));
    let mut app = TaskApp::open(processes, Privilege::User);
    app.list().invert_marks();
    app.press(KeyCode::Delete);
    let drawn = screen(&mut app);
    assert_shows(
        &drawn,
        &[
            "Do you wish to terminate the 8 processes?",
            "101 worker",
            "105 worker",
            "… and 3 more",
            "Skipped: FFM itself",
        ],
    );
    assert!(!shows(&drawn, "106 worker"));
}

#[test]
fn the_confirmation_warns_when_the_target_runs_ffm() {
    let shell = named(812, "bash");
    let ffm = ProcessSample {
        parent: Some(pid(812)),
        ..named(OWN, "ffm")
    };
    let mut app = TaskApp::open(vec![shell, ffm], Privilege::User);
    let at = app
        .list()
        .view()
        .iter()
        .position(|row| row.key() == key_of(812));
    app.list().select_index(at.unwrap(), 1);
    app.press(KeyCode::F(8));
    assert_shows(
        &screen(&mut app),
        &["FFM runs inside 812 bash; ending it ends this session."],
    );
}

/// The Send confirmation for the signal `down` rows below SIGTERM in the F9
/// menu, aimed at the shell FFM runs inside.
#[cfg(unix)]
fn sending_to_the_shell_ffm_runs_in(down: usize) -> Vec<String> {
    let ffm = ProcessSample {
        parent: Some(pid(812)),
        ..named(OWN, "ffm")
    };
    let mut app = TaskApp::open(vec![named(812, "bash"), ffm], Privilege::User);
    app.list().select_index(0, 1);
    app.press(KeyCode::F(9));
    for _ in 0..down {
        app.press(KeyCode::Down);
    }
    app.press(KeyCode::Enter);
    screen(&mut app)
}

/// SIGCONT and SIGWINCH end nothing, yet the warning that ending the shell
/// ends the session came with them too. SIGSTOP says what it does instead.
#[cfg(unix)]
#[test]
fn the_shell_warning_comes_only_with_a_signal_that_ends_or_pauses_it() {
    for (down, name) in [(7, "SIGCONT"), (12, "SIGWINCH")] {
        let drawn = sending_to_the_shell_ffm_runs_in(down);
        assert_shows(&drawn, &[&format!("Send {name} to the process?")]);
        assert!(!shows(&drawn, "FFM runs inside"), "{name}: {drawn:#?}");
    }
    assert_shows(
        &sending_to_the_shell_ffm_runs_in(6),
        &["FFM runs inside 812 bash; pausing it freezes this session."],
    );
}

#[cfg(unix)]
#[test]
fn the_send_confirmation_names_the_chosen_signal() {
    let mut app = cursor_only();
    app.press(KeyCode::F(9));
    let drawn = screen(&mut app);
    assert_shows(
        &drawn,
        &["Send signal to 4242 firefox", "SIGTERM    ask to terminate"],
    );
    app.press(KeyCode::Down);
    app.press(KeyCode::Enter);
    assert_shows(
        &screen(&mut app),
        &[
            " Send signal ",
            "Send SIGKILL to the process?",
            "{ Send } [ Cancel ]",
        ],
    );
}

#[test]
fn a_process_name_with_an_ampersand_is_shown_whole() {
    let mut app = TaskApp::open(vec![named(77, "a&b")], Privilege::User);
    app.press(KeyCode::F(8));
    assert_shows(&screen(&mut app), &["77 a&b"]);
}

#[test]
fn a_bracket_asks_a_user_before_lowering_a_priority() {
    let mut app = cursor_only();
    app.press(KeyCode::Char('['));
    assert_shows(
        &screen(&mut app),
        &[
            " Lower priority ",
            "Lower the priority of the process?",
            "4242 firefox",
            "Only root can raise it again.",
            "{ Renice } [ Cancel ]",
        ],
    );
}
