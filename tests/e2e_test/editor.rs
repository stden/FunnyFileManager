//! Purpose: Creating, editing, saving and searching files in the built-in editor.

use super::*;

/// Shift+F4 creates an empty file with the name it is given.
#[test]
fn e2e_shift_f4_creates_a_new_file() {
    let (left, right) = fixture(&[]);

    let mut events = vec![key_with(KeyCode::F(4), KeyModifiers::SHIFT)];
    events.extend(typed("fresh.txt"));
    events.push(key(KeyCode::Enter));
    events.extend(quit());
    run(left.path(), right.path(), events);

    assert!(
        left.path().join("fresh.txt").is_file(),
        "Shift+F4 creates it"
    );
}

// ─── the editor ─────────────────────────────────────────────────────────────

/// F4 opens the editor, typing changes the buffer, F2 writes it back.
#[test]
fn e2e_f4_edits_a_file_and_f2_saves_it() {
    let (left, right) = fixture(&[("edit-me.txt", "old")]);

    let mut events = vec![key(KeyCode::Down), key(KeyCode::F(4))];
    events.extend(typed("NEW "));
    events.push(key(KeyCode::F(2)));
    events.push(key(KeyCode::Esc));
    events.extend(quit());
    run(left.path(), right.path(), events);

    let saved = fs::read_to_string(left.path().join("edit-me.txt")).expect("file still there");
    assert!(
        saved.starts_with("NEW "),
        "the typed text must reach the file, got {saved:?}"
    );
}

/// Leaving a modified buffer must ask rather than discard silently.
#[test]
fn e2e_leaving_a_modified_buffer_asks_and_no_discards_the_edit() {
    let (left, right) = fixture(&[("keep.txt", "original")]);

    let mut events = vec![key(KeyCode::Down), key(KeyCode::F(4))];
    events.extend(typed("X"));
    events.push(key(KeyCode::Esc)); // opens the unsaved-changes dialog
    events.push(key(KeyCode::Char('n'))); // discard
    events.extend(quit());
    run(left.path(), right.path(), events);

    assert_eq!(
        fs::read_to_string(left.path().join("keep.txt")).unwrap(),
        "original",
        "answering No must leave the file alone"
    );
}

/// F7 inside the editor searches, and the caret lands on the match — which is
/// visible because the editor bar reports the line it is on.
#[test]
fn e2e_f7_in_the_editor_finds_a_line_and_moves_the_caret() {
    let (left, right) = fixture(&[("haystack.txt", "one\ntwo\nthree\nneedle\nfive")]);

    let mut events = vec![key(KeyCode::Down), key(KeyCode::F(4)), key(KeyCode::F(7))];
    events.extend(typed("needle"));
    events.push(key(KeyCode::Enter));
    let screen = run_until(left.path(), right.path(), events);

    let bar = screen.lines().next().expect("the editor bar");
    assert!(
        bar.contains("Ln 4/5"),
        "the caret must be on the matching line, bar was: {bar}"
    );
}

/// The F7 dialog is on screen with Far's controls.
#[test]
fn e2e_the_editor_search_dialog_offers_fars_controls() {
    let (left, right) = fixture(&[("file.txt", "content")]);

    let screen = run_until(
        left.path(),
        right.path(),
        vec![key(KeyCode::Down), key(KeyCode::F(4)), key(KeyCode::F(7))],
    );

    assert!(screen.contains("Search for"), "{screen}");
    assert!(screen.contains("Case sensitive"), "{screen}");
    assert!(screen.contains("Whole words"), "{screen}");
    assert!(screen.contains("Find previous"), "{screen}");
    assert!(screen.contains("Find next"), "{screen}");
}
