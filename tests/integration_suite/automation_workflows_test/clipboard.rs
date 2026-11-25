//! Purpose: Public isolated sessions support editor copying without sharing clipboard contents.

use super::*;

fn edit_file(session: &mut Session) {
    session.enter("left");
    session.select("file.txt");
    session.keys(&["F4"]);
}

#[test]
fn isolated_editor_copies_and_pastes_unicode_through_its_private_clipboard() {
    let root = workspace();
    fs::write(root.path().join("left/file.txt"), "a界🙂").unwrap();
    let mut session = Session::start(root.path());
    edit_file(&mut session);
    session.keys(&["Ctrl+A", "Ctrl+C", "Ctrl+End", "Ctrl+V"]);
    session.action("editor", "save");
    assert_eq!(
        fs::read_to_string(root.path().join("left/file.txt")).unwrap(),
        "a界🙂a界🙂"
    );
    session.close();
}

#[test]
fn a_second_isolated_session_cannot_paste_the_first_sessions_clipboard() {
    let first_root = workspace();
    let second_root = workspace();
    fs::write(
        first_root.path().join("left/file.txt"),
        "first-private-text",
    )
    .unwrap();
    fs::write(second_root.path().join("left/file.txt"), "second").unwrap();
    let mut first = Session::start(first_root.path());
    let mut second = Session::start(second_root.path());
    edit_file(&mut first);
    first.keys(&["Ctrl+A", "Ctrl+C"]);
    edit_file(&mut second);
    second.keys(&["Ctrl+End", "Ctrl+V"]);
    second.action("editor", "save");
    assert_eq!(
        fs::read_to_string(second_root.path().join("left/file.txt")).unwrap(),
        "second"
    );
    first.keys(&["Ctrl+End", "Ctrl+V"]);
    first.action("editor", "save");
    assert_eq!(
        fs::read_to_string(first_root.path().join("left/file.txt")).unwrap(),
        "first-private-textfirst-private-text"
    );
    first.close();
    second.close();
}
