//! Purpose: Dragging dialogs, clicks outside them and what a dialog shields.

use super::*;

/// A modal owns the screen: the panels are still drawn behind it, but they are
/// not what the pointer is over.
#[test]
fn e2e_a_click_does_not_reach_the_panels_behind_a_dialog() {
    let (left, right) = fixture(&[("safe.txt", "x")]);

    let mut events = vec![
        key(KeyCode::F(10)),     // the quit dialog is up
        click(38, SCREEN_H - 1), // a click on "5Copy" underneath it
        key(KeyCode::Char('n')), // dismiss
    ];
    events.extend(quit());
    run(left.path(), right.path(), events);

    assert!(
        !right.path().join("safe.txt").exists(),
        "a click behind a dialog must not run the key bar"
    );
}

/// A press on a blank part of the dialog takes hold of it, and a drag carries
/// it — which is how you look at what a dialog is covering.
#[test]
fn e2e_a_dialog_can_be_dragged_by_the_mouse() {
    let (left, right) = fixture(&[("a.txt", "a")]);
    let before = run_until(left.path(), right.path(), vec![key(KeyCode::F(10))]);
    let (_, title_row) = find(&before, "Quit");

    // Grab the title bar and pull it up and left.
    let (grab_x, grab_y) = (SCREEN_W / 2, title_row);
    let after = run_until(
        left.path(),
        right.path(),
        vec![
            key(KeyCode::F(10)),
            click(grab_x, grab_y),
            drag(grab_x - 6, grab_y - 3),
        ],
    );

    let (_, moved_row) = find(&after, "Quit");
    assert!(
        moved_row < title_row,
        "the dialog must follow the pointer: was row {title_row}, now {moved_row}\n{after}"
    );
}

/// Clicking the panels behind a dialog does nothing. That is what modal means,
/// and it is why the click has to be recognised as being outside rather than
/// simply falling through to the panel hit-test.
#[test]
fn e2e_clicking_outside_a_dialog_does_nothing() {
    let (left, right) = fixture(&[("a.txt", "a")]);
    let after = run_until(
        left.path(),
        right.path(),
        vec![key(KeyCode::F(10)), click(1, 1)],
    );
    assert!(
        after.contains("Do you want to quit"),
        "the dialog stays up: {after}"
    );
}

/// A dialog dragged aside does not leave the next one displaced: each opens
/// centred, where the user expects to find it.
#[test]
fn e2e_a_new_dialog_opens_centred_after_an_earlier_one_was_dragged() {
    let (left, right) = fixture(&[("a.txt", "a")]);
    let centred = run_until(left.path(), right.path(), vec![key(KeyCode::F(10))]);
    let (_, centred_row) = find(&centred, "Quit");

    let after = run_until(
        left.path(),
        right.path(),
        vec![
            // Drag the quit dialog away, dismiss it, then open it again.
            key(KeyCode::F(10)),
            click(SCREEN_W / 2, centred_row),
            drag(SCREEN_W / 2, centred_row - 4),
            key(KeyCode::Esc),
            key(KeyCode::F(10)),
        ],
    );

    let (_, reopened_row) = find(&after, "Quit");
    assert_eq!(reopened_row, centred_row, "reopened centred:\n{after}");
}
