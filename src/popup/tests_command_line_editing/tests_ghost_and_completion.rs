//! Purpose: the command line's ghost suggestion and Tab completion at the caret.

use super::*;
use crate::keymap::{tables, ActionName, Explicit};
use crate::test_support::chord;

// ─── ghost text ───────────────────────────────────────────────────────────

#[test]
fn right_accepts_the_ghost_at_the_end_but_moves_the_caret_mid_line() {
    let mut app = prompt("");
    app.history.record("cd /very/long/path");

    send(
        &mut app,
        &[key(KeyCode::Char('c')), key(KeyCode::Char('d'))],
    );
    // At the end of the line, Right takes the suggestion.
    send(&mut app, &[key(KeyCode::Right)]);
    assert_eq!(line(&app).0, "cd /very/long/path");

    // Mid-line it is an ordinary cursor move, with no ghost to compete.
    send(&mut app, &[ctrl('a'), key(KeyCode::Right)]);
    assert_eq!(line(&app), ("cd /very/long/path".to_owned(), 1));
}

/// The grey suggestion, if one is shown.
fn ghost(app: &App) -> Option<String> {
    match &app.overlay {
        Overlay::CommandLine(state) => state.ghost.clone(),
        _ => None,
    }
}

/// Whether the completion list is open.
fn list_open(app: &App) -> bool {
    matches!(&app.overlay, Overlay::CommandLine(state) if state.menu.is_some())
}

/// The prompt with `cd` typed and `cd /very/long/path` suggested after it.
fn prompt_with_ghost() -> App {
    let mut app = prompt("");
    app.history.record("cd /very/long/path");
    send(
        &mut app,
        &[key(KeyCode::Char('c')), key(KeyCode::Char('d'))],
    );
    assert!(ghost(&app).is_some(), "a suggestion applies at the end");
    app
}

/// The suggestion is taken by command, not by key code: every key of the
/// three moves towards the end of the line takes it.
#[test]
fn every_key_of_a_move_to_the_right_accepts_the_ghost_at_the_end() {
    for text in ["Right", "End", "Ctrl+Right", "Alt+Right", "Alt+F", "Ctrl+E"] {
        let mut app = prompt_with_ghost();
        send(&mut app, &[chord(text).to_event()]);
        assert_eq!(line(&app).0, "cd /very/long/path", "{text}");
    }
}

#[test]
fn right_and_end_with_a_modifier_nothing_is_bound_to_leave_the_ghost_alone() {
    for text in [
        "Shift+Right",
        "Shift+End",
        "Ctrl+End",
        "Alt+End",
        "Ctrl+Alt+Right",
    ] {
        let mut app = prompt_with_ghost();
        send(&mut app, &[chord(text).to_event()]);
        assert_eq!(line(&app), ("cd".to_owned(), 2), "{text}");
        assert!(ghost(&app).is_some(), "{text} withdrew the suggestion");
    }
}

/// Whether the command line has a selection.
fn selected(app: &App) -> bool {
    matches!(&app.overlay, Overlay::CommandLine(state) if state.line.selection().is_some())
}

/// Shift with Right or End selects, as in every field, and so does not take
/// the suggestion: with the caret at the end there is nothing to select, and
/// the line stays as it was.
#[test]
fn shift_with_right_or_end_selects_and_leaves_the_suggestion() {
    for chord in [
        KeyEvent::new(KeyCode::Right, KeyModifiers::SHIFT),
        KeyEvent::new(KeyCode::End, KeyModifiers::SHIFT),
    ] {
        let mut app = prompt_with_ghost();
        send(&mut app, &[chord]);
        assert_eq!(line(&app), ("cd".to_owned(), 2), "{chord:?}");
        assert!(ghost(&app).is_some(), "{chord:?} withdrew the suggestion");
    }

    // With a selection made from the start, Shift+End grows it to the end
    // and the suggestion is still not taken.
    let mut app = prompt_with_ghost();
    send(
        &mut app,
        &[
            key(KeyCode::Home),
            KeyEvent::new(KeyCode::End, KeyModifiers::SHIFT),
        ],
    );
    assert_eq!(line(&app), ("cd".to_owned(), 2));
    assert!(selected(&app), "Shift+End made no selection");
}

/// From the start of the line, Ctrl+E moves the caret to the end and Alt+F
/// one word on, without appending the suggestion either way; at the end of
/// the line they do take it (`every_key_of_a_move_to_the_right...`).
#[test]
fn ctrl_e_and_alt_f_move_the_caret_and_do_not_take_the_suggestion() {
    let mut app = prompt_with_ghost();
    send(&mut app, &[ctrl('a'), ctrl('e')]);
    assert_eq!(line(&app), ("cd".to_owned(), 2));

    let mut app = prompt_with_ghost();
    send(&mut app, &[ctrl('a'), alt('f')]);
    assert_eq!(line(&app).0, "cd", "Alt+F took the suggestion");
    assert_ne!(line(&app).1, 0, "Alt+F did not move the caret");
}

#[test]
fn the_ghost_follows_a_move_to_the_key_a_file_gives_it() {
    let mut app = prompt_with_ghost();
    let explicit = Explicit::from([(ActionName::new("char_right"), vec![chord("Ctrl+F")])]);
    app.keymap.command_line = tables::COMMAND_LINE.layered(&explicit).bindings;

    send(&mut app, &[key(KeyCode::Right)]);
    assert_eq!(line(&app), ("cd".to_owned(), 2), "Right is no key now");
    assert!(ghost(&app).is_some());

    send(&mut app, &[chord("Ctrl+F").to_event()]);
    assert_eq!(line(&app).0, "cd /very/long/path");
}

#[test]
fn the_ghost_is_suppressed_while_the_caret_sits_mid_line() {
    let mut app = prompt_with_ghost();

    send(&mut app, &[key(KeyCode::Left)]);
    assert_eq!(
        ghost(&app),
        None,
        "a suggestion appended after mid-line text would describe a line \
         the user is not typing"
    );
}

#[test]
fn the_ghost_is_withdrawn_while_the_list_is_open_and_returns_when_it_closes() {
    let temp = tempfile::TempDir::new().expect("temp dir");
    for name in ["Documents", "Downloads"] {
        std::fs::create_dir(temp.path().join(name)).expect("mkdir");
    }
    let mut app = prompt("cat Do");
    app.left_panel = crate::models::Panel::new(temp.path().to_path_buf()).expect("panel");
    app.active_panel = crate::models::PanelSide::Left;
    app.history.record("cat Dotfiles");

    // Two candidates open the list, and the list takes the suggestion down.
    send(&mut app, &[key(KeyCode::Tab)]);
    assert!(list_open(&app), "two candidates open the list");
    assert_eq!(ghost(&app), None, "the list and the ghost would fight");
    send(&mut app, &[key(KeyCode::Down), key(KeyCode::Up)]);
    assert_eq!(ghost(&app), None, "moving the highlight keeps it down");

    send(&mut app, &[key(KeyCode::Esc)]);
    assert!(!list_open(&app), "Esc closes the list");
    assert_eq!(
        ghost(&app).as_deref(),
        Some("tfiles"),
        "with the list gone, the history match applies again"
    );
}

mod tests_completion;
