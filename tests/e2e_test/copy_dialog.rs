//! Purpose: The copy and rename/move dialogs driven from the keyboard.

use super::*;

// ─── F5, the Copy dialog ────────────────────────────────────────────────────

/// F5 asks before copying, and offers the other panel as the default answer.
#[test]
fn e2e_f5_opens_the_copy_dialog_showing_far_s_controls() {
    let (left, right) = fixture(&[("report.txt", "data")]);

    let screen = run_until(
        left.path(),
        right.path(),
        vec![key(KeyCode::Down), key(KeyCode::F(5))],
    );

    assert!(screen.contains("Copy"), "the dialog is titled Copy");
    assert!(
        screen.contains("\"report.txt\""),
        "and names what it is about"
    );
    for control in [
        "Access rights:",
        "Default",
        "Inherit",
        "Already existing files:",
        "Ask",
        "Preserve all timestamps",
        "contents of symbolic links",
        "Process multiple destinations",
        "Use filter",
        "Cancel",
    ] {
        assert!(screen.contains(control), "the dialog must show {control:?}");
    }
}

/// Enter accepts the offered destination, which is the other panel — the same
/// thing F5 used to do outright, now as the dialog's default answer.
#[test]
fn e2e_enter_in_the_copy_dialog_copies_to_the_other_panel() {
    let (left, right) = fixture(&[("report.txt", "data")]);

    let mut events = vec![key(KeyCode::Down), key(KeyCode::F(5)), key(KeyCode::Enter)];
    events.extend(quit());
    run(left.path(), right.path(), events);

    assert!(right.path().join("report.txt").exists());
}

/// Escape leaves everything alone — the dialog is a question, not a delay.
#[test]
fn e2e_escaping_the_copy_dialog_copies_nothing() {
    let (left, right) = fixture(&[("report.txt", "data")]);

    let mut events = vec![key(KeyCode::Down), key(KeyCode::F(5)), key(KeyCode::Esc)];
    events.extend(quit());
    run(left.path(), right.path(), events);

    assert!(!right.path().join("report.txt").exists());
    assert!(left.path().join("report.txt").exists());
}

/// Press `f_key` on the first entry, clear the offered destination and type
/// `destination` instead, then accept the dialog and quit.
fn transfer_to_typed(left: &Path, right: &Path, f_key: u8, destination: &str) {
    let mut events = vec![key(KeyCode::Down), key(KeyCode::F(f_key))];
    for _ in 0..right.to_string_lossy().len() + 8 {
        events.push(key(KeyCode::Backspace));
    }
    events.extend(typed(destination));
    events.push(key(KeyCode::Enter));
    events.extend(quit());
    run(left, right, events);
}

/// The point of the dialog: a destination that is neither panel. The
/// trailing separator asks for a folder, created on the way (Far: "append
/// backslash to its name").
#[test]
fn e2e_the_copy_dialog_uses_the_typed_destination() {
    let (left, right) = fixture(&[("report.txt", "data")]);
    let elsewhere = left.path().join("elsewhere");

    let folder = format!("{}{}", elsewhere.display(), std::path::MAIN_SEPARATOR);
    transfer_to_typed(left.path(), right.path(), 5, &folder);

    assert!(
        elsewhere.join("report.txt").exists(),
        "the copy lands where it was told, creating the directory"
    );
    assert!(
        !right.path().join("report.txt").exists(),
        "and not in the other panel"
    );
}

/// F6 is "Rename or move": a new name for the one file renames it, rather
/// than making a folder of that name and moving the file inside.
#[test]
fn e2e_f6_to_a_new_name_renames_the_file() {
    let (left, right) = fixture(&[("report.txt", "data")]);
    let renamed = left.path().join("final.txt");

    transfer_to_typed(left.path(), right.path(), 6, &renamed.to_string_lossy());

    assert!(renamed.is_file(), "a file named final.txt, not a folder");
    assert_eq!(fs::read_to_string(&renamed).expect("renamed"), "data");
    assert!(
        !left.path().join("report.txt").exists(),
        "the old name is gone"
    );
}

/// Tab reaches the checkboxes and space ticks them.
#[test]
fn e2e_tab_and_space_work_the_copy_dialog_s_checkboxes() {
    let (left, right) = fixture(&[("report.txt", "data")]);

    let mut events = vec![key(KeyCode::Down), key(KeyCode::F(5))];
    // Destination → rights → existing → "Preserve all timestamps".
    events.extend([key(KeyCode::Tab), key(KeyCode::Tab), key(KeyCode::Tab)]);
    events.push(key(KeyCode::Char(' ')));
    let screen = run_until(left.path(), right.path(), events);

    assert!(
        screen.contains("[x] Preserve all timestamps"),
        "space must tick the box the cursor is on"
    );
}

/// F6 is the same dialog under a different name: Far calls moving within a
/// directory a rename, and offers one question for both.
#[test]
fn e2e_f6_opens_the_rename_move_dialog() {
    let (left, right) = fixture(&[("report.txt", "data")]);

    let screen = run_until(
        left.path(),
        right.path(),
        vec![key(KeyCode::Down), key(KeyCode::F(6))],
    );

    assert!(screen.contains("Rename/Move"), "{screen}");
    assert!(screen.contains("Rename or move"));
    assert!(
        screen.contains("Rename }") || screen.contains("{ Rename"),
        "the button says what the key does"
    );
}

/// Enter moves rather than copies: the original must be gone.
#[test]
fn e2e_the_rename_move_dialog_moves() {
    let (left, right) = fixture(&[("report.txt", "data")]);

    let mut events = vec![key(KeyCode::Down), key(KeyCode::F(6)), key(KeyCode::Enter)];
    events.extend(quit());
    run(left.path(), right.path(), events);

    assert!(right.path().join("report.txt").exists(), "it arrives");
    assert!(!left.path().join("report.txt").exists(), "and it leaves");
}

/// A move has one destination and no filter, so Far greys both. The cursor
/// must skip them: stopping on a control that does nothing is worse than not
/// drawing it.
#[test]
fn e2e_the_rename_move_dialog_skips_the_controls_it_greys() {
    let (left, right) = fixture(&[("report.txt", "data")]);

    let mut events = vec![key(KeyCode::Down), key(KeyCode::F(6))];
    // Destination → rights → existing → timestamps → symlinks → then the
    // greyed pair is skipped and the next Tab lands on the Rename button.
    events.extend(std::iter::repeat_n(key(KeyCode::Tab), 5));
    events.push(key(KeyCode::Char(' ')));
    let screen = run_until(left.path(), right.path(), events);

    assert!(
        !screen.contains("[x] Process multiple destinations"),
        "space must not tick a greyed box: {screen}"
    );
}
