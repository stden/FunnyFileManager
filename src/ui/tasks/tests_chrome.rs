//! Unit tests for the task list's chrome: the title's freshness markers,
//! the footer, the info row, the key labels and the text bars.

use std::io;
use std::time::Instant;

use crossterm::event::KeyCode;

use super::chrome::{footer, info, key_labels, title};
use super::tests::{draw, fg_of, lines, sketch};
use crate::models::{
    ControlRefusal, Notice, Owner, ProcessSample, SampleError, SystemPart, TaskList, UserScope,
};
use crate::prompt::Privilege;
use crate::test_support::processes::process;
use crate::test_support::task_list::{pid, TaskApp};
use crate::ui::palette;
use crate::ui::Cells;

fn opened(processes: Vec<ProcessSample>) -> TaskApp {
    TaskApp::open(processes, Privilege::User)
}

fn failed() -> SampleError {
    SampleError::Unreadable {
        part: SystemPart::ProcessList,
        kind: io::ErrorKind::PermissionDenied,
    }
}

#[test]
fn the_title_carries_the_most_urgent_freshness_marker() {
    let mut app = opened(vec![process(pid(10), "sh")]);
    let list = app.list();
    assert_eq!(title(list), " Task list ");
    list.toggle_hold();
    assert_eq!(title(list), " Task list · held ");
    list.mark_overdue();
    assert_eq!(title(list), " Task list · stale ");
    list.toggle_hold();
    let now = Instant::now();
    list.take_request(now);
    list.apply_sample(Err(failed()), now);
    list.toggle_hold();
    assert_eq!(title(list), " Task list · refresh failed ");
    list.mark_stalled();
    assert_eq!(title(list), " Task list · not responding ");
}

#[test]
fn a_list_with_no_table_has_no_marker_but_the_hold() {
    let mut list = TaskList::default();
    assert_eq!(title(&list), " Task list ");
    list.mark_stalled();
    assert_eq!(title(&list), " Task list ");
    list.toggle_hold();
    assert_eq!(title(&list), " Task list · held ");
}

fn bobs_and_alices() -> Vec<ProcessSample> {
    let alice = ProcessSample {
        owner: Owner::Named("alice".to_owned()),
        ..process(pid(30), "vim")
    };
    vec![process(pid(10), "firefox"), process(pid(20), "bash"), alice]
}

#[test]
fn the_footer_counts_what_is_shown_and_says_what_narrows_it() {
    let mut app = opened(bobs_and_alices());
    assert_eq!(footer(app.list()).as_deref(), Some(" 3 processes "));
    app.list().toggle_mark(1);
    assert_eq!(
        footer(app.list()).as_deref(),
        Some(" 3 processes, 1 marked ")
    );
    app.list().set_filter("fire|vi");
    assert_eq!(
        footer(app.list()).as_deref(),
        Some(" 2 of 3 processes, 1 marked, filter fire|vi ")
    );
    app.list()
        .set_scope(UserScope::Only(Owner::Named("alice".to_owned())));
    assert_eq!(
        footer(app.list()).as_deref(),
        Some(" 1 of 3 processes, filter fire|vi, user alice ")
    );
}

#[test]
fn the_footer_names_one_process_in_the_singular_and_counts_the_unreadable() {
    let mut app = opened(vec![process(pid(10), "sh")]);
    assert_eq!(footer(app.list()).as_deref(), Some(" 1 process "));
    let mut unreadable = crate::test_support::processes::one_core(Vec::new());
    unreadable.unreadable = 3;
    app.apply(unreadable);
    assert_eq!(
        footer(app.list()).as_deref(),
        Some(" 0 processes, 3 unreadable ")
    );
    assert_eq!(footer(&TaskList::default()), None);
}

#[test]
fn the_info_row_shows_the_cursor_process_until_a_notice_replaces_it() {
    let mut app = opened(vec![ProcessSample {
        command: "/usr/bin/firefox --new-window".to_owned(),
        ..process(pid(4242), "firefox")
    }]);
    let row = info(app.list(), Cells::new(80)).to_string();
    assert_eq!(row, "4242 firefox: /usr/bin/firefox --new-window");
    let clipped = info(app.list(), Cells::new(20)).to_string();
    assert_eq!(clipped, "4242 firefox: /usr/}");
    app.list()
        .set_notice(Notice::Refused(ControlRefusal::OwnProcess));
    let refused = info(app.list(), Cells::new(80));
    assert_eq!(refused.to_string(), "This is FFM itself; quit with F10.");
    assert_eq!(refused.spans[0].style.bg, Some(palette::WARNING_BG));
    app.list().set_notice(Notice::Marked {
        changed: 2,
        total: 2,
    });
    let marked = info(app.list(), Cells::new(80));
    assert_eq!(marked.spans[0].style.bg, Some(palette::DIALOG_BG));
    assert_eq!(info(&TaskList::default(), Cells::new(80)).to_string(), "");
}

#[test]
fn the_info_row_makes_a_hostile_command_line_printable() {
    let mut app = opened(vec![ProcessSample {
        command: "evil\u{1b}[2J\u{202e}txt".to_owned(),
        ..process(pid(66), "evil\u{7}")
    }]);
    let row = info(app.list(), Cells::new(80)).to_string();
    assert_eq!(row, "66 evil\u{FFFD}: evil\u{FFFD}[2J\u{FFFD}txt");
}

#[test]
fn the_key_labels_name_what_each_toggle_switches_to() {
    let mut app = opened(Vec::new());
    let labels = key_labels(app.list());
    let names: Vec<&str> = labels.iter().map(|(_, label)| *label).collect();
    assert_eq!(
        names,
        [
            "Help", "Name", "Info", "Filter", "Tree", "Sort", "Nice", "Kill", "Signal", "Quit",
            "Hold", ""
        ]
    );
    let list = app.list();
    list.toggle_command_view();
    list.toggle_tree();
    list.toggle_hold();
    let labels = key_labels(list);
    assert_eq!(
        [labels[1].1, labels[4].1, labels[10].1],
        ["Cmd", "List", "Live"]
    );
}

#[test]
fn the_filter_bar_replaces_the_key_bar_with_its_caret_after_the_text() {
    let mut app = sketch();
    app.press(KeyCode::F(4));
    app.type_text("fire|chr");
    let mut terminal = draw(&mut app, 80, 24);
    let bottom = lines(terminal.backend().buffer())[23].clone();
    assert_eq!(
        bottom.trim_end(),
        " Enter Done  Esc Clear  Filter: fire|chr"
    );
    let caret = terminal.get_cursor_position().unwrap();
    assert_eq!((caret.x, caret.y), (40, 23));
}

#[test]
fn the_search_bar_draws_a_query_that_matches_nothing_in_the_failed_colour() {
    let mut app = sketch();
    app.press(KeyCode::Char('/'));
    app.type_text("fire");
    let terminal = draw(&mut app, 80, 24);
    let buffer = terminal.backend().buffer();
    let bottom = lines(buffer)[23].clone();
    assert_eq!(
        bottom.trim_end(),
        " F3 Next  Enter Done  Esc Cancel  Search: fire"
    );
    assert_eq!(fg_of(buffer, 23, "fire"), palette::DATA);
    app.type_text("zzz");
    let terminal = draw(&mut app, 80, 24);
    assert_eq!(
        fg_of(terminal.backend().buffer(), 23, "firezzz"),
        palette::PROMPT_FAILED
    );
}

#[test]
fn the_pid_bar_shows_the_digits_typed() {
    let mut app = sketch();
    app.type_text("42");
    let terminal = draw(&mut app, 80, 24);
    let bottom = lines(terminal.backend().buffer())[23].clone();
    assert_eq!(bottom.trim_end(), " Enter Done  Esc Cancel  PID: 42");
}

#[test]
fn a_long_filter_scrolls_so_the_caret_stays_on_the_row() {
    let mut app = sketch();
    app.press(KeyCode::F(4));
    app.type_text(&"x".repeat(100));
    let mut terminal = draw(&mut app, 40, 12);
    let bottom = lines(terminal.backend().buffer())[11].clone();
    assert!(bottom.ends_with("…xxxxxx "), "{bottom:?}");
    let caret = terminal.get_cursor_position().unwrap();
    assert_eq!((caret.x, caret.y), (39, 11));
}
