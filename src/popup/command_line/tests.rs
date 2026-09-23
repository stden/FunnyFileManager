//! Unit tests: the command line's reverse search, and the guard every helper
//! here carries.
//!
//! Each helper begins by asking whether the command line is still the overlay
//! on screen, because it is called from a dispatcher that may have closed it —
//! a `submit` that opens a dialog, a menu row that changes the mode. Every one
//! of those guards is a branch, and a helper that acted anyway would write into
//! an overlay that is no longer there.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tempfile::TempDir;

use super::*;
use crate::history::RingIndex;
use crate::models::{App, CommandLine, Overlay, Panel};

fn app_at(dir: &TempDir) -> App {
    let mut app = App::new().expect("app");
    app.left_panel = Panel::new(dir.path().to_path_buf()).expect("left");
    app.right_panel = Panel::new(dir.path().to_path_buf()).expect("right");
    app
}

fn ctrl(ch: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(ch), KeyModifiers::CONTROL)
}

fn plain(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::empty())
}

fn typed(app: &App) -> String {
    let Overlay::CommandLine(state) = &app.overlay else {
        panic!("the command line is open");
    };
    state.line.text().to_owned()
}

/// A command line with `history` behind it, ready to search.
fn app_with_history(dir: &TempDir, history: &[&str]) -> App {
    let mut app = app_at(dir);
    for line in history {
        app.history.record(line);
    }
    app.set_overlay(Overlay::CommandLine(CommandLine::default()));
    app
}

// ─── the guards ─────────────────────────────────────────────────────────────

/// Every helper must be inert once the command line is no longer the overlay.
/// They are called from a dispatcher that can have closed it a moment earlier,
/// and one that acted anyway would write into a dialog that is gone.
#[test]
fn every_helper_does_nothing_once_the_command_line_has_closed() {
    let dir = TempDir::new().expect("temp");
    let mut app = app_at(&dir);
    app.set_overlay(Overlay::Help);

    assert!(state(&app).is_none());
    assert!(state_mut(&mut app).is_none());
    assert!(!menu_open(&app));
    assert!(!ghost_acceptable(&app));
    assert_eq!(next_search_start(&app), RingIndex::NEWEST);

    // None of these may panic, and none may change the overlay. The closures
    // panic on purpose: reaching one would mean the guard let it through.
    close_menu(&mut app);
    with_menu(&mut app, |_| panic!("there is no menu"));
    edit(&mut app, |_| panic!("there is no buffer"));
    move_caret(&mut app, |_| panic!("there is no caret"));
    edit_and_refresh(&mut app, |_| panic!("there is no buffer"));
    edit_query(&mut app, |_| panic!("there is no query"));
    refresh_after_edit(&mut app);
    history_step(&mut app, Step::Older);
    restore(&mut app, "anything");
    start_search(&mut app);
    search_from(&mut app, RingIndex::NEWEST);
    finish_search(&mut app);
    cancel_search(&mut app);
    complete_at_cursor(&mut app);
    refresh_menu(&mut app);
    accept_ghost(&mut app);
    refresh_ghost(&mut app);

    assert!(matches!(app.overlay, Overlay::Help), "still the help page");
}

// ─── reverse search ─────────────────────────────────────────────────────────

#[test]
fn ctrl_r_finds_the_newest_match_and_steps_to_older_ones() {
    let dir = TempDir::new().expect("temp");
    let mut app = app_with_history(&dir, &["git status", "git log", "cargo test"]);

    process(&mut app, ctrl('r'));
    // "git", not "g": `cargo` contains a g too, and the newest match wins.
    for ch in "git".chars() {
        process(&mut app, plain(KeyCode::Char(ch)));
    }
    assert_eq!(typed(&app), "git log", "the newest match first");

    process(&mut app, ctrl('r'));
    assert_eq!(typed(&app), "git status", "then the one before it");
}

/// A query that matches nothing leaves the buffer on the previous match rather
/// than blanking it — what bash does, minus the beep.
#[test]
fn a_query_that_matches_nothing_keeps_the_last_match_on_screen() {
    let dir = TempDir::new().expect("temp");
    let mut app = app_with_history(&dir, &["git status"]);

    process(&mut app, ctrl('r'));
    for ch in "git".chars() {
        process(&mut app, plain(KeyCode::Char(ch)));
    }
    assert_eq!(typed(&app), "git status");

    process(&mut app, plain(KeyCode::Char('z')));
    assert_eq!(typed(&app), "git status", "the buffer is not blanked");

    process(&mut app, plain(KeyCode::Backspace));
    assert_eq!(typed(&app), "git status", "and backspace searches again");
}

/// Ctrl+G is bash's abort, and Ctrl+C and Esc get the same treatment: the
/// buffer goes back to what was being typed before the search started.
#[test]
fn every_abort_key_puts_the_typed_line_back() {
    for abort in [ctrl('g'), ctrl('c'), plain(KeyCode::Esc)] {
        let dir = TempDir::new().expect("temp");
        let mut app = app_with_history(&dir, &["git status"]);
        for ch in "ec".chars() {
            process(&mut app, plain(KeyCode::Char(ch)));
        }

        process(&mut app, ctrl('r'));
        for ch in "git".chars() {
            process(&mut app, plain(KeyCode::Char(ch)));
        }
        assert_eq!(typed(&app), "git status", "the search took over");

        process(&mut app, abort);
        assert_eq!(typed(&app), "ec", "{abort:?} restored the typed line");
    }
}

/// `cancel_search` is only ever invoked from `search_key`, which `process`
/// only routes to while `Recall::Searching` is already the state — so its own
/// "not searching" guard is otherwise unreachable. Called directly, with the
/// command line open but no search running, to cover that branch.
#[test]
fn cancel_search_does_nothing_when_no_search_is_running() {
    let dir = TempDir::new().expect("temp");
    let mut app = app_with_history(&dir, &["git status"]);
    for ch in "abc".chars() {
        process(&mut app, plain(KeyCode::Char(ch)));
    }
    assert_eq!(typed(&app), "abc");

    cancel_search(&mut app);

    assert_eq!(typed(&app), "abc", "the guard must leave the buffer alone");
}

/// Enter during a search keeps the match and leaves the search.
#[test]
fn enter_ends_the_search_keeping_the_match() {
    let dir = TempDir::new().expect("temp");
    let mut app = app_with_history(&dir, &["cargo test"]);

    process(&mut app, ctrl('r'));
    process(&mut app, plain(KeyCode::Char('c')));
    process(&mut app, plain(KeyCode::Enter));

    assert_eq!(typed(&app), "cargo test");
}

/// A key the search has no use for is swallowed rather than typed into the
/// query, or every stray function key would break the match.
#[test]
fn a_key_the_search_has_no_use_for_leaves_it_alone() {
    let dir = TempDir::new().expect("temp");
    let mut app = app_with_history(&dir, &["cargo test"]);

    process(&mut app, ctrl('r'));
    process(&mut app, plain(KeyCode::Char('c')));
    process(&mut app, plain(KeyCode::F(5)));

    assert_eq!(typed(&app), "cargo test");
}

#[test]
fn completion_is_idle_when_the_setting_is_off() {
    let dir = TempDir::new().expect("temp");
    let mut app = app_with_history(&dir, &["git status"]);
    app.settings
        .set(crate::menu::SettingsPage::AutoComplete, "off")
        .expect("setting");
    process(&mut app, plain(KeyCode::Char('g')));
    process(&mut app, plain(KeyCode::Tab));
    assert_eq!(typed(&app), "g");
}
