//! Purpose: Tests for the task list's text bars: the live filter kept by
//! Enter and cleared by Esc, the search that moves the cursor and wraps, the
//! PID jump, the keys that end a bar and then act, and pastes into a bar.

use crossterm::event::{KeyCode, KeyModifiers};

use crate::models::{Found, Modal};
use crate::popup::paste_text;
use crate::prompt::Privilege;
use crate::test_support::task_list::{named, TaskApp};

use super::tests::{chord, cursor_pid, press, three, type_text};

mod tests_pid_jump_and_paste;

fn filter_line(tasks: &mut TaskApp) -> String {
    tasks.list().options().filter.line().to_owned()
}

fn search_found(tasks: &mut TaskApp) -> Found {
    let Modal::Search(bar) = tasks.list().modal() else {
        panic!("the search bar is open");
    };
    bar.found()
}

#[test]
fn the_filter_edits_live_and_enter_keeps_it() {
    let mut tasks = three();
    press(&mut tasks, KeyCode::F(4));
    type_text(&mut tasks, "nginq");
    assert_eq!(tasks.shown(), Vec::<u32>::new());
    press(&mut tasks, KeyCode::Backspace);
    assert_eq!(tasks.shown(), [30]);
    type_text(&mut tasks, "x");
    type_text(&mut tasks, "|vim");
    assert_eq!(tasks.shown(), [10, 30]);
    press(&mut tasks, KeyCode::Enter);
    assert_eq!(tasks.list().modal(), &Modal::Browse);
    assert_eq!(filter_line(&mut tasks), "nginx|vim");
    press(&mut tasks, KeyCode::Char('\\'));
    let Modal::Filter(bar) = tasks.list().modal() else {
        panic!("the filter bar reopens");
    };
    assert_eq!(bar.line().text(), "nginx|vim", "on the filter in force");
}

#[test]
fn esc_clears_the_filter_and_the_list_keys_still_move() {
    let mut tasks = three();
    press(&mut tasks, KeyCode::F(4));
    type_text(&mut tasks, "o|x");
    assert_eq!(tasks.shown(), [20, 30]);
    press(&mut tasks, KeyCode::Down);
    assert_eq!(cursor_pid(&mut tasks), Some(30));
    assert!(
        matches!(tasks.list().modal(), Modal::Filter(_)),
        "still open"
    );
    press(&mut tasks, KeyCode::Left);
    assert_eq!(
        filter_line(&mut tasks),
        "o|x",
        "a caret move filters nothing"
    );
    press(&mut tasks, KeyCode::Esc);
    assert_eq!(filter_line(&mut tasks), "");
    assert_eq!(tasks.shown(), [10, 20, 30]);
    assert_eq!(tasks.list().modal(), &Modal::Browse);
}

#[test]
fn the_search_moves_to_the_first_match_and_f3_to_the_next_wrapping() {
    let mut tasks = TaskApp::open(
        vec![named(10, "vim"), named(20, "python3"), named(30, "python2")],
        Privilege::User,
    );
    press(&mut tasks, KeyCode::Char('/'));
    assert_eq!(search_found(&mut tasks), Found::Match);
    type_text(&mut tasks, "PYTH");
    assert_eq!(cursor_pid(&mut tasks), Some(20));
    press(&mut tasks, KeyCode::F(3));
    assert_eq!(cursor_pid(&mut tasks), Some(30));
    press(&mut tasks, KeyCode::F(3));
    assert_eq!(cursor_pid(&mut tasks), Some(20), "wrapped");
    type_text(&mut tasks, "z");
    assert_eq!(search_found(&mut tasks), Found::NoMatch);
    assert_eq!(cursor_pid(&mut tasks), Some(20), "the cursor stays");
    press(&mut tasks, KeyCode::Enter);
    assert_eq!(tasks.list().modal(), &Modal::Browse);
    assert_eq!(cursor_pid(&mut tasks), Some(20));
}

#[test]
fn alt_letter_opens_the_search_with_that_letter_searched_for() {
    let mut tasks = three();
    chord(&mut tasks, KeyCode::Char('g'), KeyModifiers::ALT);
    assert_eq!(cursor_pid(&mut tasks), Some(30));
    let Modal::Search(bar) = tasks.list().modal() else {
        panic!("the search bar is open");
    };
    assert_eq!(bar.line().text(), "g");
}

#[test]
fn a_list_key_ends_the_search_and_then_moves() {
    let mut tasks = three();
    press(&mut tasks, KeyCode::Char('/'));
    type_text(&mut tasks, "pyth");
    press(&mut tasks, KeyCode::Home);
    assert!(
        matches!(tasks.list().modal(), Modal::Search(_)),
        "a caret key"
    );
    press(&mut tasks, KeyCode::Down);
    assert_eq!(tasks.list().modal(), &Modal::Browse);
    assert_eq!(cursor_pid(&mut tasks), Some(30));
    press(&mut tasks, KeyCode::Char('/'));
    press(&mut tasks, KeyCode::F(8));
    assert!(
        matches!(tasks.list().modal(), Modal::Search(_)),
        "F8 does nothing in the bar"
    );
    press(&mut tasks, KeyCode::Esc);
    assert_eq!(tasks.list().modal(), &Modal::Browse);
}

/// With nothing typed there is nothing to find again: F3 leaves the cursor.
#[test]
fn f3_with_an_empty_search_leaves_the_cursor() {
    let mut tasks = three();
    press(&mut tasks, KeyCode::Char('/'));
    press(&mut tasks, KeyCode::F(3));
    assert_eq!(cursor_pid(&mut tasks), Some(10));
    assert_eq!(search_found(&mut tasks), Found::Match);
}
