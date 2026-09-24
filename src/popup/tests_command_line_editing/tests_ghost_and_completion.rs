//! Purpose: the command line's ghost suggestion and Tab completion at the caret.

use super::*;

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

#[test]
fn the_ghost_is_suppressed_while_the_caret_sits_mid_line() {
    let mut app = prompt("");
    app.history.record("cd /very/long/path");
    send(
        &mut app,
        &[key(KeyCode::Char('c')), key(KeyCode::Char('d'))],
    );

    let ghost_at_end = match &app.overlay {
        Overlay::CommandLine(state) => state.ghost.clone(),
        _ => None,
    };
    assert!(ghost_at_end.is_some(), "a suggestion applies at the end");

    send(&mut app, &[key(KeyCode::Left)]);
    let ghost_mid_line = match &app.overlay {
        Overlay::CommandLine(state) => state.ghost.clone(),
        _ => None,
    };
    assert_eq!(
        ghost_mid_line, None,
        "a suggestion appended after mid-line text would describe a line \
         the user is not typing"
    );
}

// ─── completion at the caret ──────────────────────────────────────────────

#[test]
#[expect(
    clippy::string_slice,
    reason = "a caret off a char boundary panics the test, which fails it as loudly as the assertion"
)]
fn tab_completes_the_token_under_the_caret_and_keeps_the_tail() {
    let temp = tempfile::TempDir::new().expect("temp dir");
    std::fs::create_dir(temp.path().join("Documents")).expect("mkdir");

    let mut app = prompt("cat Doc /etc/hosts");
    app.left_panel = crate::models::Panel::new(temp.path().to_path_buf()).expect("panel");
    app.active_panel = crate::models::PanelSide::Left;

    // Park the caret right after "Doc", with the second argument to its right.
    send(&mut app, &[ctrl('a')]);
    for _ in 0.."cat Doc".len() {
        send(&mut app, &[key(KeyCode::Right)]);
    }
    send(&mut app, &[key(KeyCode::Tab)]);

    let (text, cursor) = line(&app);
    assert_eq!(text, "cat Documents/ /etc/hosts");
    assert_eq!(
        &text[..cursor],
        "cat Documents/",
        "the caret stays at the end of what was just completed"
    );
}
