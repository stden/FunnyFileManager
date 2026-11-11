//! Purpose: unit tests for the wrapping search and the PID jump.

use super::*;

fn commands() -> TaskList {
    let command = |raw: u32, name: &str, line: &str| ProcessSample {
        command: line.to_owned(),
        ..process(raw, name)
    };
    listing(vec![
        command(1, "python3", "python3 /srv/x/train.py"),
        command(2, "bash", "-bash"),
        command(3, "python3", "python3 /srv/x/serve.py"),
    ])
}

#[test]
fn search_starts_at_the_cursor_and_wraps() {
    let mut list = commands();
    assert_eq!(list.find("PYTH", SearchFrom::Cursor, PAGE), Found::Match);
    assert_eq!(cursor_pid(&list), Some(1), "the cursor's own row matches");
    assert_eq!(
        list.find("pyth", SearchFrom::AfterCursor, PAGE),
        Found::Match
    );
    assert_eq!(cursor_pid(&list), Some(3));
    assert_eq!(
        list.find("pyth", SearchFrom::AfterCursor, PAGE),
        Found::Match
    );
    assert_eq!(cursor_pid(&list), Some(1), "wrapped past the end");
    assert_eq!(list.find("perl", SearchFrom::Cursor, PAGE), Found::NoMatch);
    assert_eq!(cursor_pid(&list), Some(1), "a miss leaves the cursor");
}

#[test]
fn search_reads_the_last_column_as_shown() {
    let mut list = commands();
    assert_eq!(list.find("serve", SearchFrom::Cursor, PAGE), Found::Match);
    list.toggle_command_view();
    assert_eq!(list.command_view(), CommandView::Name);
    assert_eq!(list.find("serve", SearchFrom::Cursor, PAGE), Found::NoMatch);
}

#[test]
fn a_pid_jump_lands_only_on_a_shown_pid() {
    let mut list = twenty();
    assert_eq!(list.jump_to_pid(pid(17), PAGE), Found::Match);
    assert_eq!(cursor_pid(&list), Some(17));
    assert_eq!(list.offset(), 12);
    assert_eq!(list.jump_to_pid(pid(21), PAGE), Found::NoMatch);
    assert_eq!(cursor_pid(&list), Some(17));
}
