//! Purpose: The "Ask" choice: a taken name stops the copy at the "File already exists" question, and each answer does what it says.

use super::*;
use crate::models::{ClashAnswer, Overlay};

/// A copy of `notes.txt` ("new") to a folder already holding one ("old"),
/// stopped at the question.
fn asked() -> (App, TempDir, TempDir) {
    let (mut app, from, to) = app_with(&[("notes.txt", b"new")]);
    fs::write(to.path().join("notes.txt"), b"old").expect("test setup");
    mark(&mut app, "notes.txt");
    copy_with(&mut app, &settings(to.path()));
    (app, from, to)
}

fn read(dir: &TempDir, name: &str) -> Option<Vec<u8>> {
    fs::read(dir.path().join(name)).ok()
}

/// The default asks, and nothing is touched or renamed while it does.
#[test]
fn a_taken_name_asks_before_anything_is_copied() {
    let (app, _from, to) = asked();

    let Overlay::ExistingFile(question) = &app.overlay else {
        panic!("expected the question, got {:?}", app.status);
    };
    let clash = question.clash().expect("one clash");
    assert_eq!(clash.dest, to.path().join("notes.txt"));
    assert_eq!(clash.incoming.size, Some(3));
    assert_eq!(clash.existing.size, Some(3));
    assert_eq!(read(&to, "notes.txt").as_deref(), Some(&b"old"[..]));
    assert_eq!(read(&to, "notes_1.txt"), None, "no numbered copy");
}

#[test]
fn overwrite_replaces_the_file_already_there() {
    let (mut app, from, to) = asked();

    answer_existing(&mut app, ClashAnswer::Overwrite);

    assert!(matches!(app.overlay, Overlay::None));
    assert_eq!(read(&to, "notes.txt").as_deref(), Some(&b"new"[..]));
    assert_eq!(read(&to, "notes_1.txt"), None);
    assert!(from.path().join("notes.txt").exists(), "a copy, not a move");
}

#[test]
fn skip_leaves_the_file_already_there() {
    let (mut app, _from, to) = asked();

    answer_existing(&mut app, ClashAnswer::Skip);

    assert_eq!(read(&to, "notes.txt").as_deref(), Some(&b"old"[..]));
    assert_eq!(read(&to, "notes_1.txt"), None);
}

#[test]
fn keep_both_puts_the_copy_beside_under_a_free_name() {
    let (mut app, _from, to) = asked();

    answer_existing(&mut app, ClashAnswer::KeepBoth);

    assert_eq!(read(&to, "notes.txt").as_deref(), Some(&b"old"[..]));
    assert_eq!(read(&to, "notes_1.txt").as_deref(), Some(&b"new"[..]));
    assert!(matches!(app.status, Status::Transferred { renamed: 1, .. }));
}

#[test]
fn cancel_copies_nothing_and_says_so() {
    let (mut app, _from, to) = asked();

    cancel_existing(&mut app);

    assert!(matches!(app.overlay, Overlay::None));
    assert_eq!(read(&to, "notes.txt").as_deref(), Some(&b"old"[..]));
    assert_eq!(app.status, Status::TransferCancelled(Purpose::Copy));
}

/// Two clashes are asked one at a time, and only the answered ones and the
/// free names are copied when the last answer comes.
#[test]
fn each_clash_gets_its_own_answer_and_free_names_are_not_asked_about() {
    let (mut app, _from, to) = app_with(&[
        ("a.txt", b"new a"),
        ("b.txt", b"new b"),
        ("c.txt", b"new c"),
    ]);
    fs::write(to.path().join("a.txt"), b"old a").expect("test setup");
    fs::write(to.path().join("b.txt"), b"old b").expect("test setup");
    for name in ["a.txt", "b.txt", "c.txt"] {
        mark(&mut app, name);
    }
    copy_with(&mut app, &settings(to.path()));

    let Overlay::ExistingFile(question) = &app.overlay else {
        panic!("expected the question");
    };
    assert_eq!(question.position(), (1, 2), "c.txt is free");
    answer_existing(&mut app, ClashAnswer::Overwrite);
    assert!(
        matches!(app.overlay, Overlay::ExistingFile(_)),
        "b.txt still to ask"
    );
    assert_eq!(
        read(&to, "c.txt"),
        None,
        "nothing copied before the last answer"
    );
    answer_existing(&mut app, ClashAnswer::Skip);

    assert_eq!(read(&to, "a.txt").as_deref(), Some(&b"new a"[..]));
    assert_eq!(read(&to, "b.txt").as_deref(), Some(&b"old b"[..]));
    assert_eq!(read(&to, "c.txt").as_deref(), Some(&b"new c"[..]));
}

/// "Remember choice" answers every clash still to come.
#[test]
fn remember_choice_answers_the_rest_at_once() {
    let (mut app, _from, to) = app_with(&[("a.txt", b"new a"), ("b.txt", b"new b")]);
    fs::write(to.path().join("a.txt"), b"old a").expect("test setup");
    fs::write(to.path().join("b.txt"), b"old b").expect("test setup");
    mark(&mut app, "a.txt");
    mark(&mut app, "b.txt");
    copy_with(&mut app, &settings(to.path()));

    let Overlay::ExistingFile(question) = &mut app.overlay else {
        panic!("expected the question");
    };
    question.remember = true;
    answer_existing(&mut app, ClashAnswer::Overwrite);

    assert!(matches!(app.overlay, Overlay::None));
    assert_eq!(read(&to, "a.txt").as_deref(), Some(&b"new a"[..]));
    assert_eq!(read(&to, "b.txt").as_deref(), Some(&b"new b"[..]));
}

/// The keys: `O` overwrites, Esc cancels — the way a user answers.
#[test]
fn the_dialog_answers_to_its_hotkeys() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    let (mut app, _from, to) = asked();
    let key = |code| KeyEvent::new(code, KeyModifiers::empty());
    assert!(crate::popup::process_popup_input(
        &mut app,
        key(KeyCode::Char('o'))
    ));
    assert_eq!(read(&to, "notes.txt").as_deref(), Some(&b"new"[..]));

    let (mut app, _from, to) = asked();
    assert!(crate::popup::process_popup_input(
        &mut app,
        key(KeyCode::Esc)
    ));
    assert_eq!(read(&to, "notes.txt").as_deref(), Some(&b"old"[..]));
    assert_eq!(app.status, Status::TransferCancelled(Purpose::Copy));
}

/// A move asks too, and Overwrite moves the file over the one there.
#[test]
fn a_move_asks_and_overwrite_moves_over_the_old_file() {
    let (mut app, from, to) = app_with(&[("notes.txt", b"new")]);
    fs::write(to.path().join("notes.txt"), b"old").expect("test setup");
    mark(&mut app, "notes.txt");
    let mut dialog = settings(to.path());
    dialog.purpose = Purpose::Move;
    copy_with(&mut app, &dialog);
    assert!(matches!(app.overlay, Overlay::ExistingFile(_)));

    answer_existing(&mut app, ClashAnswer::Overwrite);

    assert_eq!(read(&to, "notes.txt").as_deref(), Some(&b"new"[..]));
    assert!(!from.path().join("notes.txt").exists(), "moved");
}

/// A copy onto itself is not a question: no answer could make it work.
#[test]
fn a_copy_onto_itself_is_refused_not_asked_about() {
    let (mut app, from, _to) = app_with(&[("notes.txt", b"new")]);
    mark(&mut app, "notes.txt");

    copy_with(&mut app, &settings(from.path()));

    assert!(!matches!(app.overlay, Overlay::ExistingFile(_)));
    assert_eq!(read(&from, "notes_1.txt"), None);
}
