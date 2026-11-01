//! Purpose: What an import stores: all or nothing, within the limit, and never over the user's own.

use super::*;

#[test]
fn an_import_that_cannot_be_saved_is_not_taken_in() {
    let (dir, mut app) = setup();
    fs::write(dir.path().join(FILE_NAME), "{").expect("write");
    app.sessions = SessionStore::load_from(dir.path().join(FILE_NAME));
    assert!(matches!(
        run(&mut app, Some(dir.path().join("demo.netbox"))),
        Err(SessionError::Store(_))
    ));
    assert!(app.sessions.is_empty());
    assert_eq!(transcript(&app).len(), 0, "nothing is reported as imported");
}

fn large_export() -> String {
    use std::fmt::Write;

    let mut text = String::from("<NetBox><Sessions>");
    let padding = "x".repeat(450);
    for index in 0..1000 {
        write!(
            text,
            "<Session name=\"{index:04}{padding}\"><HostName>h.test</HostName>\
             <PublicKeyFile>{padding}</PublicKeyFile></Session>"
        )
        .expect("write XML");
    }
    text.push_str("</Sessions></NetBox>");
    text
}

#[test]
fn an_import_exceeding_the_store_limit_preserves_committed_sessions() {
    let (dir, mut app) = setup();
    app.sessions = file_store(dir.path(), vec![session("kept.test", None)]);
    let previous = app.sessions.clone();
    let saved_path = dir.path().join(FILE_NAME);
    let saved_bytes = fs::read(&saved_path).expect("previous file");
    let export = large_export();
    assert!(u64::try_from(export.len()).expect("fits") < MAX_FILE_BYTES);
    let path = dir.path().join("large.netbox");
    fs::write(&path, export).expect("write export");

    assert!(matches!(
        run(&mut app, Some(path)),
        Err(SessionError::Store(StoreError::TooLarge {
            limit: MAX_FILE_BYTES
        }))
    ));

    assert_eq!(app.sessions, previous);
    assert_eq!(fs::read(saved_path).expect("kept file"), saved_bytes);
    assert_eq!(
        SessionStore::load_from(dir.path().join(FILE_NAME)),
        previous
    );
    assert_eq!(transcript(&app).len(), 0, "no successful import report");
}

#[test]
fn an_import_keeps_the_users_own_session_of_the_same_name() {
    let (dir, mut app) = setup();
    let mut mine = session("mine.test", None);
    mine.name = name("web");
    app.sessions = file_store(dir.path(), vec![mine]);
    run(&mut app, None).expect("imported");
    assert_eq!(
        app.sessions.find(&name("web")).map(|s| s.host.to_string()),
        Some("mine.test".to_owned())
    );
    assert_eq!(app.status_text(), "NetBox: 0 imported, 2 skipped");
}

/// With no saved password in the file there is nothing to say about one.
#[test]
fn an_import_without_passwords_says_nothing_about_them() {
    let (dir, mut app) = setup();
    fs::write(
        dir.path().join("plain.netbox"),
        "<NetBox><Sessions><Session><HostName>h.test</HostName></Session></Sessions></NetBox>",
    )
    .expect("write");
    run(&mut app, Some(dir.path().join("plain.netbox"))).expect("imported");
    assert_eq!(transcript(&app).len(), 0, "{:?}", transcript(&app));
    assert_eq!(app.status_text(), "NetBox: 1 imported, 0 skipped");
}
