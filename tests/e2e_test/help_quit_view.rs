//! Purpose: Help, the viewer and the quit confirmation.

use super::*;

// ─── overlays ───────────────────────────────────────────────────────────────

/// F1 opens help and F10 closes it — F10 leaves whatever is in front of you.
#[test]
fn e2e_f1_opens_help_and_f10_closes_it() {
    let (left, right) = fixture(&[("a.txt", "x")]);

    let mut events = vec![key(KeyCode::F(1)), key(KeyCode::F(10))];
    events.extend(quit());
    let screen = run(left.path(), right.path(), events);

    assert!(
        !screen.contains("Far Manager style navigation"),
        "help must be closed again:\n{screen}"
    );
    assert!(
        screen.contains("1Help"),
        "and the panels are back:\n{screen}"
    );
}

/// F10 asks before quitting, so a stray press cannot end the session.
#[test]
fn e2e_f10_asks_before_quitting_and_no_returns_to_the_panels() {
    let (left, right) = fixture(&[("still-here.txt", "x")]);

    // Stop while the answer is still on screen: a sequence that went on to
    // quit would leave the final frame showing the dialog it had just
    // confirmed, and prove nothing about dismissing it.
    let screen = run_until(
        left.path(),
        right.path(),
        vec![key(KeyCode::F(10)), key(KeyCode::Char('n'))],
    );

    assert!(
        screen.contains("still-here"),
        "answering No must return to the panels:\n{screen}"
    );
    assert!(
        !screen.contains("Do you want to quit"),
        "and the dialog is gone:\n{screen}"
    );
}

/// F10 followed by Yes really does end the session — which every other test
/// here depends on, so it is worth asserting once directly.
#[test]
fn e2e_f10_then_yes_ends_the_session() {
    let (left, right) = fixture(&[]);
    let screen = run_until(left.path(), right.path(), vec![key(KeyCode::F(10))]);
    assert!(
        screen.contains("Do you want to quit"),
        "F10 asks first:\n{screen}"
    );

    // `run` would panic if the loop did not return on its own.
    run(left.path(), right.path(), quit());
}

/// F3 views a file without being able to change it.
#[test]
fn e2e_f3_views_a_files_contents() {
    let (left, right) = fixture(&[("readme.txt", "VIEWABLE CONTENT")]);

    let screen = run_until(
        left.path(),
        right.path(),
        vec![key(KeyCode::Down), key(KeyCode::F(3))],
    );

    assert!(screen.contains("VIEWABLE CONTENT"), "{screen}");
    assert_eq!(
        fs::read_to_string(left.path().join("readme.txt")).unwrap(),
        "VIEWABLE CONTENT",
        "viewing must not change the file"
    );
}

/// F10 twice must quit. It asked the question, so it answers it: closing the
/// dialog instead made the key its own undo, and a reader feeding F10 forever
/// — which is what a stuck key is — never reached the end of the session.
#[test]
fn e2e_pressing_f10_twice_quits() {
    let (left, right) = fixture(&[("a.txt", "a")]);
    run(
        left.path(),
        right.path(),
        vec![key(KeyCode::F(10)), key(KeyCode::F(10))],
    );
}

/// …and Escape still cancels, so the dialog is a real question.
#[test]
fn e2e_escape_cancels_the_quit_dialog() {
    let (left, right) = fixture(&[("a.txt", "a")]);
    let screen = run_until(
        left.path(),
        right.path(),
        vec![key(KeyCode::F(10)), key(KeyCode::Esc)],
    );
    // The panel splits name and extension into columns, so the file is not
    // one string on screen — the dialog being gone is the thing to assert.
    assert!(
        !screen.contains("Do you want to quit"),
        "Escape must dismiss the dialog: {screen}"
    );
}
