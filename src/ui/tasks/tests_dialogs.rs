//! Unit tests for what the task list opens over its table: the kill and
//! renice confirmations, the Priority form, process information, the Error
//! dialog, the mask dialog, the help page, and the dialog tracker.

use crossterm::event::KeyCode;

use super::tests::{draw, fg_of, lines, sketch};
use crate::models::{ControlError, Modal, ProcessSample};
use crate::prompt::Privilege;
use crate::test_support::processes::process;
use crate::test_support::task_list::{key_of, named, pid, TaskApp, OWN};
use crate::ui::palette;

/// The sketch with nothing marked, so a dialog is about the cursor's
/// process, 4242 firefox.
fn cursor_only() -> TaskApp {
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
fn screen(app: &mut TaskApp) -> Vec<String> {
    let terminal = draw(app, 80, 24);
    lines(terminal.backend().buffer())
}

/// Whether some row of `screen` contains `text`.
fn shows(screen: &[String], text: &str) -> bool {
    screen.iter().any(|row| row.contains(text))
}

#[track_caller]
fn assert_shows(screen: &[String], texts: &[&str]) {
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
fn the_priority_form_disables_set_while_the_value_is_not_a_nice_value() {
    let mut app = cursor_only();
    app.press(KeyCode::F(7));
    let terminal = draw(&mut app, 80, 24);
    let buffer = terminal.backend().buffer();
    let drawn = lines(buffer);
    assert_shows(
        &drawn,
        &[
            " Priority ",
            "4242 firefox",
            "Nice:",
            "-20 runs first, 19 runs last",
        ],
    );
    let buttons = drawn.iter().position(|row| row.contains("Set }")).unwrap();
    let y = u16::try_from(buttons).unwrap();
    assert_ne!(fg_of(buffer, y, "Set"), palette::DISABLED_FG);
    app.press(KeyCode::Backspace);
    app.type_text("x");
    let terminal = draw(&mut app, 80, 24);
    assert_eq!(
        fg_of(terminal.backend().buffer(), y, "Set"),
        palette::DISABLED_FG
    );
}

#[test]
fn the_priority_form_puts_the_caret_where_typing_lands() {
    let mut app = cursor_only();
    app.press(KeyCode::F(7));
    app.type_text("5");
    app.press(KeyCode::Left);
    let mut terminal = draw(&mut app, 80, 24);
    let caret = terminal.get_cursor_position().unwrap();
    let row = &lines(terminal.backend().buffer())[usize::from(caret.y)];
    let before: String = row.chars().take(usize::from(caret.x)).collect();
    assert!(before.ends_with("Nice:       0"), "{before:?}");
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

#[test]
fn process_information_comes_from_the_snapshot() {
    let mut app = cursor_only();
    app.press(KeyCode::Enter);
    assert_shows(
        &screen(&mut app),
        &[
            " Process information ",
            "PID           4242",
            "Parent        -",
            "Name          firefox",
            "Command       /usr/lib/firefox/firefox -contentproc -childID 12 ",
            "              -isForBrowser 7 tab",
            "User          bob",
            "State         R (running)",
            "Priority/Nice 20 / 0",
            "Threads       80",
            "Running for   -",
            "CPU time      12:04.55",
            "VIRT/RES/SHR  3.1 G / 612 M / 1.0 M",
        ],
    );
}

#[test]
fn a_long_command_line_is_wrapped_to_four_rows_and_cut() {
    let long = ProcessSample {
        command: "x".repeat(500),
        ..process(pid(9), "long")
    };
    let mut app = TaskApp::open(vec![long], Privilege::User);
    app.press(KeyCode::F(3));
    let drawn = screen(&mut app);
    let command_rows = drawn
        .iter()
        .filter(|row| row.contains("xxxxxxxxxx"))
        .count();
    assert_eq!(command_rows, 4);
    assert_shows(&drawn, &[&format!("{}}}", "x".repeat(49))]);
}

#[test]
fn information_on_a_process_that_left_says_so() {
    let mut app = TaskApp::open(vec![named(9, "gone")], Privilege::User);
    app.press(KeyCode::Enter);
    app.resample(Vec::new());
    assert!(matches!(app.list().modal(), Modal::Info(_)));
    assert_shows(&screen(&mut app), &["Process 9 has exited."]);
}

#[test]
fn a_failed_delivery_is_listed_in_the_error_dialog() {
    let mut app = cursor_only();
    app.host.fail(key_of(4242), ControlError::PermissionDenied);
    app.press(KeyCode::F(8));
    app.press(KeyCode::Enter);
    assert_shows(
        &screen(&mut app),
        &[
            " Error ",
            "4242 firefox belongs to another user (bob).",
            "[ OK ]",
        ],
    );
}

#[test]
fn the_mask_dialog_is_the_panels_own() {
    let mut app = cursor_only();
    app.press(KeyCode::Char('+'));
    assert_shows(&screen(&mut app), &[" Select ", "Enter the mask:"]);
    assert!(app.app.dialog_mouse.area.is_some());
}

#[test]
fn the_help_page_replaces_the_table_rows_and_scrolls() {
    let mut app = cursor_only();
    app.press(KeyCode::F(1));
    let terminal = draw(&mut app, 80, 24);
    let buffer = terminal.backend().buffer();
    let drawn = lines(buffer);
    assert!(
        drawn[7].starts_with("║ Up, Down, PgUp, PgDn "),
        "{}",
        drawn[7]
    );
    assert_eq!(buffer[(2, 7)].bg, palette::HELP_BG);
    assert!(app.app.dialog_mouse.area.is_none());
    app.press(KeyCode::Down);
    let drawn = screen(&mut app);
    assert!(drawn[7].starts_with("║ Home, End "), "{}", drawn[7]);
}

#[test]
fn browsing_records_no_dialog_for_the_next_click() {
    let mut app = cursor_only();
    app.press(KeyCode::F(8));
    draw(&mut app, 80, 24);
    assert!(app.app.dialog_mouse.area.is_some());
    app.press(KeyCode::Esc);
    draw(&mut app, 80, 24);
    assert!(app.app.dialog_mouse.area.is_none());
}
