//! Unit tests for the task list's bottom row: the key labels and the
//! filter, search and PID bars that replace the key bar.

use crossterm::event::KeyCode;

use super::chrome::key_labels;
use super::tests::{draw, fg_of, lines, sketch};
use super::tests_chrome::opened;
use crate::ui::palette;

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
