//! Purpose: How failures reach the status bar, and a store that did not load.

use super::*;

#[test]
fn a_failure_reaches_the_status_bar_under_netbox_s_name() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app_in(dir.path());
    app.set_status(Status::CommandOutput("before".to_owned()));
    report(&mut app, Ok(()));
    assert_eq!(app.status_text(), "before");
    report(&mut app, Err(SessionError::NotFound(name("x"))));
    assert_eq!(
        app.status,
        Status::Error("NetBox: no session named x".to_owned())
    );
}

#[test]
fn every_error_reads_as_one_line() {
    let texts = [
        (SessionError::NotFound(name("a")), "no session named a"),
        (
            SessionError::NameTaken(name("a")),
            "a session named a already exists",
        ),
        (
            SessionError::Invalid(InvalidField::new(Field::Port, Problem::NotAPort)),
            "Port: must be a number from 1 to 65535",
        ),
        (
            SessionError::Import(ImportError::NotNetBox),
            "not a NetBox session export",
        ),
        (
            SessionError::NoImportSource,
            "name a .netbox file, or put a file panel's cursor on one",
        ),
    ];
    for (error, text) in texts {
        assert_eq!(error.to_string(), text);
    }
    let store = SessionError::Store(StoreError::Damaged);
    assert_eq!(store.to_string(), StoreError::Damaged.to_string());
    let from: SessionError = InvalidField::new(Field::Name, Problem::Empty).into();
    assert!(matches!(from, SessionError::Invalid(_)));
}

#[test]
fn installing_a_store_that_did_not_load_says_so() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app_in(dir.path());
    app.set_status(Status::CommandOutput("before".to_owned()));
    install(
        &mut app,
        file_store(dir.path(), vec![session("a.test", None)]),
    );
    assert_eq!(app.sessions.len(), 1);
    assert_eq!(app.status_text(), "before", "a good store says nothing");

    fs::write(dir.path().join(FILE_NAME), "{").expect("write");
    install(
        &mut app,
        SessionStore::load_from(dir.path().join(FILE_NAME)),
    );
    assert!(app.sessions.is_damaged());
    assert_eq!(
        app.status,
        Status::Error(format!("NetBox: {}", StoreError::Damaged))
    );
}
