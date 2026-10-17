//! Purpose: Ctrl+R's reverse search — newest match first and older on each press, a miss keeping the last match, every abort key restoring the typed line.

use super::*;

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
