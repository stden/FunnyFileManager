//! Purpose: Isolated session state stays private and both close and EOF reap the process.

use super::*;

#[test]
fn two_isolated_sessions_do_not_share_panels_or_modals() {
    let root = workspace();
    let mut first = Session::start(root.path());
    let mut second = Session::start(root.path());
    first.enter("left");
    first.keys(&["F1"]);
    let first_state = first.state();
    let second_state = second.state();
    assert_eq!(first_state["modal"]["kind"], "help");
    assert_eq!(second_state["modal"]["kind"], "none");
    assert_path(
        &first_state["panels"]["left"]["path"],
        &root.path().join("left"),
    );
    assert_path(&second_state["panels"]["left"]["path"], root.path());
    first.close();
    second.close();
}

#[test]
fn eof_stops_the_process_without_a_close_request_or_terminal() {
    let root = workspace();
    let mut session = Session::start(root.path());
    session.keys(&["F1"]);
    session.eof();
    assert_eq!(
        fs::read(root.path().join("left/file.txt")).unwrap(),
        b"alpha\n"
    );
}

#[test]
fn close_stops_even_with_an_unsaved_editor_and_preserves_disk_bytes() {
    let root = workspace();
    let mut session = Session::start(root.path());
    session.enter("left");
    session.select("file.txt");
    session.keys(&["F4"]);
    let state = session.text("unsaved ");
    assert_eq!(state["editor"]["modified"], true);
    session.close();
    assert_eq!(
        fs::read(root.path().join("left/file.txt")).unwrap(),
        b"alpha\n"
    );
}
