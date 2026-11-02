//! Purpose: Copy, move and delete from the keyboard, and where the cursor lands after.

use super::*;

// ─── file operations ────────────────────────────────────────────────────────

/// F5 copies the entry under the cursor to the other panel. The assertion is
/// the file on disk, which is the only thing that matters afterwards.
#[test]
fn e2e_f5_copies_the_selected_file_to_the_other_panel() {
    let (left, right) = fixture(&[("payload.txt", "contents")]);

    // F5 opens the Copy dialog; Enter takes the offered destination, which is
    // the other panel.
    let mut events = vec![key(KeyCode::Down), key(KeyCode::F(5)), key(KeyCode::Enter)];
    events.extend(quit());
    run(left.path(), right.path(), events);

    let copied = right.path().join("payload.txt");
    assert!(copied.exists(), "F5 must copy to the other panel");
    assert_eq!(fs::read_to_string(copied).unwrap(), "contents");
    assert!(
        left.path().join("payload.txt").exists(),
        "a copy leaves the original"
    );
}

/// F6 moves rather than copies: the original must be gone.
#[test]
fn e2e_f6_moves_the_selected_file_to_the_other_panel() {
    let (left, right) = fixture(&[("moving.txt", "contents")]);

    let mut events = vec![key(KeyCode::Down), key(KeyCode::F(6)), key(KeyCode::Enter)];
    events.extend(quit());
    run(left.path(), right.path(), events);

    assert!(right.path().join("moving.txt").exists(), "moved in");
    assert!(
        !left.path().join("moving.txt").exists(),
        "a move leaves nothing behind"
    );
}

/// F8 must NOT delete on its own — the confirmation is the whole point.
#[test]
fn e2e_f8_asks_before_deleting_and_cancelling_keeps_the_file() {
    let (left, right) = fixture(&[("precious.txt", "keep me")]);

    let mut events = vec![key(KeyCode::Down), key(KeyCode::F(8)), key(KeyCode::Esc)];
    events.extend(quit());
    run(left.path(), right.path(), events);

    assert!(
        left.path().join("precious.txt").exists(),
        "cancelling the dialog must keep the file"
    );
}

/// …and confirming it does delete.
#[test]
fn e2e_f8_deletes_once_confirmed() {
    let (left, right) = fixture(&[("doomed.txt", "bye")]);

    let mut events = vec![
        key(KeyCode::Down),
        key(KeyCode::F(8)),
        key(KeyCode::Char('y')),
    ];
    events.extend(quit());
    run(left.path(), right.path(), events);

    assert!(
        !left.path().join("doomed.txt").exists(),
        "confirming must delete the file"
    );
}

/// The delete dialog names the file it is about, on Far's red ground.
#[test]
fn e2e_the_delete_dialog_names_the_file() {
    let (left, right) = fixture(&[("named.txt", "x")]);

    let screen = run_until(
        left.path(),
        right.path(),
        vec![key(KeyCode::Down), key(KeyCode::F(8))],
    );

    assert!(screen.contains("Delete"), "the dialog is titled:\n{screen}");
    assert!(
        screen.contains("Do you wish to delete the file?"),
        "and asks:\n{screen}"
    );
    assert!(screen.contains("named.txt"), "naming the file:\n{screen}");
    assert!(screen.contains("{ Delete }"), "default button:\n{screen}");
    assert!(screen.contains("[ Cancel ]"), "cancel button:\n{screen}");
}

// ─── marking ────────────────────────────────────────────────────────────────

/// Insert marks the entry under the cursor and moves on, so a run of files can
/// be marked by holding one key — and F5 then copies all of them.
#[test]
fn e2e_marked_files_are_copied_together() {
    let (left, right) = fixture(&[("one.txt", "1"), ("two.txt", "2"), ("three.txt", "3")]);

    let mut events = vec![
        key(KeyCode::Down),
        key(KeyCode::Insert),
        key(KeyCode::Insert),
        key(KeyCode::F(5)),
        key(KeyCode::Enter),
    ];
    events.extend(quit());
    run(left.path(), right.path(), events);

    let copied = fs::read_dir(right.path())
        .expect("read right")
        .filter_map(Result::ok)
        .count();
    assert_eq!(copied, 2, "both marked files must be copied");
}

/// Deleting a file leaves the cursor where it was, so the next file is under
/// it. Far works this way, and it is what makes deleting several files in a
/// row possible: jumping back to the top after each one means hunting for the
/// place again every time.
///
/// Asserted by deleting twice without moving. If the cursor stayed put, the
/// second delete takes the neighbour; if it sprang back to the top it lands on
/// "..", which deletes nothing at all.
#[test]
fn e2e_the_cursor_stays_put_after_a_delete() {
    let (left, right) = fixture(&[
        ("a.txt", "a"),
        ("b.txt", "b"),
        ("c.txt", "c"),
        ("d.txt", "d"),
    ]);

    // Two Downs: past "..", past "a.txt", onto "b.txt".
    let mut events = vec![key(KeyCode::Down), key(KeyCode::Down)];
    events.push(key(KeyCode::F(8)));
    events.push(key(KeyCode::Char('y')));
    events.push(key(KeyCode::F(8)));
    events.push(key(KeyCode::Char('y')));
    events.extend(quit());
    run(left.path(), right.path(), events);

    assert!(!left.path().join("b.txt").exists(), "the first delete");
    assert!(
        !left.path().join("c.txt").exists(),
        "the second delete must take c.txt, which moved into b.txt's row — a \
         cursor thrown back to the top would sit on \"..\" and delete nothing"
    );
    assert!(
        left.path().join("a.txt").exists() && left.path().join("d.txt").exists(),
        "and nothing else is touched"
    );
}
