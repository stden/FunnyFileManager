//! Purpose: Clicking the buttons and checkboxes of the quit, delete, copy and find dialogs.

use super::*;

/// Clicking Yes must quit, exactly as pressing Y does. The mouse was ignored
/// entirely while a dialog was up, so every button in the program was
/// keyboard-only.
#[test]
fn e2e_clicking_yes_in_the_quit_dialog_quits() {
    let (left, right) = fixture(&[("a.txt", "a")]);
    let screen = run_until(left.path(), right.path(), vec![key(KeyCode::F(10))]);
    let (column, row) = find(&screen, "Yes");

    // No trailing quit(): if the click does not quit, the loop runs out of
    // events and the test fails on that.
    run(
        left.path(),
        right.path(),
        vec![key(KeyCode::F(10)), click(column, row)],
    );
}

/// …and clicking No must not.
#[test]
fn e2e_clicking_no_in_the_quit_dialog_returns_to_the_panels() {
    let (left, right) = fixture(&[("a.txt", "a")]);
    let screen = run_until(left.path(), right.path(), vec![key(KeyCode::F(10))]);
    let (column, row) = find(&screen, "No");

    let after = run_until(
        left.path(),
        right.path(),
        vec![key(KeyCode::F(10)), click(column, row)],
    );
    assert!(
        !after.contains("Do you want to quit"),
        "No dismisses the dialog: {after}"
    );
}

/// The delete dialog is the one where a misclick costs the most, so its
/// buttons are worth their own test.
#[test]
fn e2e_clicking_cancel_in_the_delete_dialog_keeps_the_file() {
    let (left, right) = fixture(&[("precious.txt", "keep")]);
    let screen = run_until(
        left.path(),
        right.path(),
        vec![key(KeyCode::Down), key(KeyCode::F(8))],
    );
    let (column, row) = find(&screen, "Cancel");

    let mut events = vec![key(KeyCode::Down), key(KeyCode::F(8)), click(column, row)];
    events.extend(quit());
    run(left.path(), right.path(), events);

    assert!(left.path().join("precious.txt").exists());
}

/// A checkbox answers to the click that reaches it.
#[test]
fn e2e_clicking_a_checkbox_in_the_copy_dialog_ticks_it() {
    let (left, right) = fixture(&[("a.txt", "a")]);
    let screen = run_until(
        left.path(),
        right.path(),
        vec![key(KeyCode::Down), key(KeyCode::F(5))],
    );
    let (column, row) = find(&screen, "[ ] Preserve all timestamps");

    let after = run_until(
        left.path(),
        right.path(),
        vec![key(KeyCode::Down), key(KeyCode::F(5)), click(column, row)],
    );
    assert!(
        after.contains("[x] Preserve all timestamps"),
        "the click must tick the box: {after}"
    );
}

/// A button click names the button to run; it must not accidentally execute
/// whichever control happened to have keyboard focus before the click.
#[test]
fn e2e_clicking_copy_runs_copy_even_when_cancel_had_focus() {
    let (left, right) = fixture(&[("mouse-copy.txt", "copy me")]);
    let mut open = vec![key(KeyCode::Down), key(KeyCode::F(5))];
    // Destination -> ... -> Copy -> Cancel.
    open.extend(std::iter::repeat_n(key(KeyCode::Tab), 8));

    let screen = run_until(left.path(), right.path(), open.clone());
    let (column, row) = find(&screen, "{ Copy }");

    open.push(click(column, row));
    open.extend(quit());
    run(left.path(), right.path(), open);

    assert!(
        right.path().join("mouse-copy.txt").exists(),
        "clicking Copy must copy even after Cancel held keyboard focus"
    );
}

/// Search direction buttons also name an action independent of the old focus.
/// Clicking Find previous from the query field must run backwards immediately.
#[test]
fn e2e_clicking_find_previous_runs_the_named_search() {
    let body = "first needle\nmiddle\nlast needle";
    let (left, right) = fixture(&[("search.txt", body)]);
    let mut open = vec![key(KeyCode::Down), key(KeyCode::F(4)), key(KeyCode::F(7))];
    open.extend(typed("needle"));

    let screen = run_until(left.path(), right.path(), open.clone());
    let (column, row) = find(&screen, "Find previous");
    open.push(click(column, row));

    let after = run_until(left.path(), right.path(), open);
    assert!(
        !after.contains("Search for") && after.contains("Ln 3/3"),
        "Find previous must close the dialog and select the last match:\n{after}"
    );
}
