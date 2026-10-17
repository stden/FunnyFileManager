//! Unit tests: each answer reaches its operation, and a failure reaches the
//! status bar.

use tempfile::TempDir;

use super::{confirm_question, submit_input};
use crate::models::{InputPurpose, QuestionPurpose};
use crate::test_support::netbox::{app_in, name, session};

#[test]
fn the_copy_name_copies_the_session_and_a_taken_one_is_refused() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app_in(dir.path());
    app.sessions.upsert(session("a.test", None));
    submit_input(
        &mut app,
        InputPurpose::DuplicateSession(name("a.test")),
        "b",
    );
    assert!(app.sessions.find(&name("b")).is_some());
    submit_input(
        &mut app,
        InputPurpose::DuplicateSession(name("a.test")),
        "b",
    );
    assert_eq!(
        app.status_text(),
        "Error: NetBox: a session named b already exists"
    );
}

#[test]
fn an_import_path_is_imported_or_its_failure_reported() {
    let dir = TempDir::new().expect("temp dir");
    std::fs::write(
        dir.path().join("x.netbox"),
        "<NetBox><Sessions><Session><HostName>h.test</HostName></Session></Sessions></NetBox>",
    )
    .expect("write");
    let mut app = app_in(dir.path());
    submit_input(
        &mut app,
        InputPurpose::ImportNetBox,
        &dir.path().join("x.netbox").display().to_string(),
    );
    assert!(app.sessions.find(&name("h.test")).is_some());
    submit_input(&mut app, InputPurpose::ImportNetBox, "missing.netbox");
    assert!(
        app.status_text()
            .starts_with("Error: NetBox: cannot read the file: "),
        "{}",
        app.status_text()
    );
}

#[test]
fn yes_to_the_delete_question_deletes_and_a_missing_session_is_reported() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app_in(dir.path());
    app.sessions.upsert(session("a.test", None));
    confirm_question(&mut app, QuestionPurpose::DeleteSession(name("a.test")));
    assert!(app.sessions.is_empty());
    confirm_question(&mut app, QuestionPurpose::DeleteSession(name("a.test")));
    assert_eq!(app.status_text(), "Error: NetBox: no session named a.test");
}
