//! Unit tests for the task list's forms and pages over its table: the
//! Priority form, process information, the Error dialog, the mask dialog,
//! the help page, and the dialog tracker.

use crossterm::event::KeyCode;
use ratatui::style::Style;

use super::tests::{draw, fg_of, lines};
use super::tests_dialogs::{assert_shows, cursor_only, screen};
use crate::models::{ControlError, Modal, ProcessSample};
use crate::prompt::Privilege;
use crate::test_support::processes::process;
use crate::test_support::task_list::{key_of, named, pid, TaskApp};
use crate::ui::palette;

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
    let unavailable = palette::disabled_style(Style::default().bg(palette::DIALOG_BG))
        .fg
        .unwrap();
    assert_ne!(fg_of(buffer, y, "Set"), unavailable);
    app.press(KeyCode::Backspace);
    app.type_text("x");
    let terminal = draw(&mut app, 80, 24);
    assert_eq!(fg_of(terminal.backend().buffer(), y, "Set"), unavailable);
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

/// Two marked, one refused: the dialog says what was sent before why the
/// rest was not.
#[test]
fn a_partly_failed_delivery_says_what_was_sent_first() {
    let mut app = TaskApp::open(
        vec![named(1002, "sleep"), named(4242, "firefox")],
        Privilege::User,
    );
    app.host.fail(key_of(4242), ControlError::PermissionDenied);
    app.press(KeyCode::Char('*'));
    app.press(KeyCode::F(8));
    app.press(KeyCode::Enter);
    let drawn = screen(&mut app);
    assert_shows(
        &drawn,
        &[
            "Sent SIGTERM to 1 process.",
            "4242 firefox belongs to another user",
        ],
    );
    let row = |text: &str| drawn.iter().position(|line| line.contains(text));
    assert!(row("Sent SIGTERM") < row("4242 firefox belongs"));
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
