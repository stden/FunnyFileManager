//! Purpose: unit tests for the dialog and a session agreeing in both
//! directions, blank fields taking defaults, and a bad field naming itself.

use super::*;

fn stored() -> Session {
    let mut session = Session::new(
        Protocol::Sftp,
        HostName::new("example.test").expect("host"),
        Port::new(2222).expect("port"),
        Some(UserName::new("demo").expect("user")),
    );
    session.auth = Auth::KeyFile(KeyFile::new("/home/bob/.ssh/id").expect("key"));
    session.remote_dir = Some(RemotePath::parse("/var/www").expect("dir"));
    session
}

fn typed(host: &str) -> SessionEdit {
    SessionEdit {
        host: host.to_owned(),
        ..SessionEdit::create()
    }
}

#[test]
fn a_session_fills_the_dialog_and_comes_back_unchanged() {
    let session = stored();
    let dialog = SessionEdit::from_session(&session);
    assert_eq!(dialog.port, "2222");
    assert_eq!(dialog.user, "demo");
    assert_eq!(dialog.auth, AuthKind::KeyFile);
    assert_eq!(dialog.key_file, "/home/bob/.ssh/id");
    assert_eq!(dialog.remote_dir, "/var/www");
    assert_eq!(dialog.field(), SessionField::Name);
    assert_eq!(dialog.purpose(), &EditPurpose::Edit(session.name.clone()));
    assert_eq!(dialog.to_session(), Ok(session));
}

#[test]
fn a_saved_session_is_the_users_own_whatever_it_was_imported_from() {
    let mut imported = stored();
    imported.origin = Origin::NetBoxFile("/srv/x/demo.netbox".into());
    let saved = SessionEdit::from_session(&imported)
        .to_session()
        .expect("valid");
    assert_eq!(saved.origin, Origin::Manual);
}

#[test]
fn blank_fields_take_their_defaults() {
    let dialog = SessionEdit {
        protocol: Protocol::Ftp,
        port: " ".to_owned(),
        auth: AuthKind::Agent,
        ..typed("example.test")
    };
    let session = dialog.to_session().expect("valid");
    assert_eq!(session.name.as_str(), "example.test");
    assert_eq!(session.port, Port::FTP);
    assert_eq!(session.user, None);
    assert_eq!(session.auth, Auth::Agent);
    assert_eq!(session.remote_dir, None);
    assert_eq!(SessionEdit::create().port, "22");
    assert_eq!(SessionEdit::create().purpose(), &EditPurpose::Create);
}

#[test]
fn a_typed_name_wins_over_the_default() {
    let dialog = SessionEdit {
        name: " web ".to_owned(),
        user: "demo".to_owned(),
        ..typed("example.test")
    };
    assert_eq!(
        dialog.to_session().map(|s| s.name.to_string()),
        Ok("web".to_owned())
    );
    let unnamed = SessionEdit {
        user: "demo".to_owned(),
        ..typed("example.test")
    };
    assert_eq!(
        unnamed.to_session().map(|s| s.name.to_string()),
        Ok("demo@example.test".to_owned())
    );
}

#[test]
fn the_first_bad_field_is_named() {
    let refusal = |dialog: SessionEdit| dialog.to_session().expect_err("refused");
    let fail = |field, problem| InvalidField::new(field, problem);
    assert_eq!(refusal(typed("")), fail(Field::Host, Problem::Empty));
    let port = SessionEdit {
        port: "http".to_owned(),
        ..typed("h")
    };
    assert_eq!(refusal(port), fail(Field::Port, Problem::NotAPort));
    let user = SessionEdit {
        user: "-x".to_owned(),
        ..typed("h")
    };
    assert_eq!(refusal(user), fail(Field::User, Problem::LeadingDash));
    let dir = SessionEdit {
        remote_dir: "www".to_owned(),
        ..typed("h")
    };
    assert_eq!(refusal(dir), fail(Field::RemoteDir, Problem::NotAbsolute));
    let key = SessionEdit {
        auth: AuthKind::KeyFile,
        ..typed("h")
    };
    assert_eq!(refusal(key), fail(Field::KeyFile, Problem::Empty));
}

#[test]
fn a_name_is_checked_like_any_other_field() {
    let name = SessionEdit {
        name: "a\u{7}".to_owned(),
        ..typed("h")
    };
    assert_eq!(
        name.to_session(),
        Err(InvalidField::new(Field::Name, Problem::ControlCharacter))
    );
}
