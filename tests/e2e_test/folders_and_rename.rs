//! Purpose: Make-folder and in-place rename dialogs.

use super::*;

// ─── naming things ──────────────────────────────────────────────────────────

/// F7 asks for a name, as Far does, instead of inventing `new_folder`.
#[test]
fn e2e_f7_asks_for_the_folder_name_and_uses_it() {
    let (left, right) = fixture(&[]);

    let mut events = vec![key(KeyCode::F(7))];
    events.extend(typed("chosen-name"));
    events.push(key(KeyCode::Enter));
    events.extend(quit());
    run(left.path(), right.path(), events);

    assert!(
        left.path().join("chosen-name").is_dir(),
        "the typed name must be the folder's name"
    );
    assert!(
        !left.path().join("new_folder").exists(),
        "and nothing is invented alongside it"
    );
}

#[test]
fn e2e_escaping_the_folder_dialog_creates_nothing() {
    let (left, right) = fixture(&[]);

    let mut events = vec![key(KeyCode::F(7))];
    events.extend(typed("abandoned"));
    events.push(key(KeyCode::Esc));
    events.extend(quit());
    run(left.path(), right.path(), events);

    assert_eq!(
        fs::read_dir(left.path()).unwrap().count(),
        0,
        "cancelling must leave the directory empty"
    );
}

/// Shift+F6 renames where the file is, rather than moving it anywhere.
#[test]
fn e2e_shift_f6_renames_the_selected_file_in_place() {
    let (left, right) = fixture(&[("before.txt", "same bytes")]);

    let mut events = vec![
        key(KeyCode::Down),
        key_with(KeyCode::F(6), KeyModifiers::SHIFT),
    ];
    // The dialog opens on the current name, so clear it first.
    events.extend(std::iter::repeat_n(
        key(KeyCode::Backspace),
        "before.txt".len(),
    ));
    events.extend(typed("after.txt"));
    events.push(key(KeyCode::Enter));
    events.extend(quit());
    run(left.path(), right.path(), events);

    assert!(left.path().join("after.txt").exists(), "renamed");
    assert!(!left.path().join("before.txt").exists(), "old name is gone");
    assert_eq!(
        fs::read_to_string(left.path().join("after.txt")).unwrap(),
        "same bytes",
        "a rename must not touch the contents"
    );
    assert!(
        !right.path().join("after.txt").exists(),
        "and must not move it to the other panel — that is F6"
    );
}

/// The dialog opens on the current name, so a small edit is a small edit.
#[test]
fn e2e_the_rename_dialog_opens_on_the_current_name() {
    let (left, right) = fixture(&[("report.txt", "x")]);

    let screen = run_until(
        left.path(),
        right.path(),
        vec![
            key(KeyCode::Down),
            key_with(KeyCode::F(6), KeyModifiers::SHIFT),
        ],
    );

    assert!(screen.contains("Rename"), "the dialog is titled:\n{screen}");
    assert!(
        screen.contains("Rename report.txt to:"),
        "and says what it is renaming:\n{screen}"
    );
}

/// Renaming onto an existing name must refuse rather than overwrite.
#[test]
fn e2e_a_rename_onto_an_existing_name_is_refused() {
    let (left, right) = fixture(&[("keep.txt", "original"), ("other.txt", "other")]);

    let mut events = vec![
        key(KeyCode::Down),
        key(KeyCode::Down), // land on other.txt (after "..", keep.txt)
        key_with(KeyCode::F(6), KeyModifiers::SHIFT),
    ];
    events.extend(std::iter::repeat_n(
        key(KeyCode::Backspace),
        "other.txt".len(),
    ));
    events.extend(typed("keep.txt"));
    events.push(key(KeyCode::Enter));
    events.extend(quit());
    run(left.path(), right.path(), events);

    assert_eq!(
        fs::read_to_string(left.path().join("keep.txt")).unwrap(),
        "original",
        "the existing file must survive"
    );
}
