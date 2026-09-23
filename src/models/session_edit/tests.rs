//! Unit tests: the dialog and a session agree in both directions, a blank
//! field takes its default, a bad one names itself, and the cursor walks the
//! controls that apply.

use super::{EditPurpose, SessionEdit, SessionField};
use crate::models::{
    Auth, AuthKind, Field, HostName, InvalidField, KeyFile, Origin, Port, Problem, Protocol,
    RemotePath, Session, UserName,
};

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

/// Tab until the cursor is on `field`; one lap of the ring at most, so a
/// broken Tab fails the test instead of hanging it.
fn tab_to(dialog: &mut SessionEdit, field: SessionField) {
    for _ in 0..SessionField::ALL.len() {
        if dialog.field() == field {
            return;
        }
        dialog.tab(true);
    }
    panic!("Tab never reached {field:?}");
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

const RING_WITHOUT_KEY_FILE: [SessionField; 9] = [
    SessionField::Name,
    SessionField::Protocol,
    SessionField::Host,
    SessionField::Port,
    SessionField::User,
    SessionField::Auth,
    SessionField::RemoteDir,
    SessionField::Ok,
    SessionField::Cancel,
];

#[test]
fn tab_walks_the_controls_in_a_ring_past_the_key_file() {
    let mut dialog = SessionEdit::create();
    for expected in RING_WITHOUT_KEY_FILE {
        assert_eq!(dialog.field(), expected);
        dialog.tab(true);
    }
    assert_eq!(dialog.field(), SessionField::Name, "the ring closes");
    dialog.tab(false);
    assert_eq!(dialog.field(), SessionField::Cancel);
    assert!(dialog.on_cancel());
    assert!(!SessionEdit::create().is_enabled(SessionField::KeyFile));
    assert!(SessionEdit::create().is_enabled(SessionField::Host));
}

#[test]
fn tab_stops_at_the_key_file_while_the_login_is_by_key() {
    let mut by_key = SessionEdit {
        auth: AuthKind::KeyFile,
        ..SessionEdit::create()
    };
    for _ in 0..6 {
        by_key.tab(true);
    }
    assert_eq!(by_key.field(), SessionField::KeyFile);
    by_key.tab(true);
    by_key.tab(false);
    assert_eq!(by_key.field(), SessionField::KeyFile, "and back");
    by_key.push('k');
    assert_eq!(by_key.key_file, "k");
}

#[test]
fn typing_edits_the_text_field_with_the_cursor() {
    let mut dialog = SessionEdit::create();
    dialog.push('w');
    dialog.push('b');
    dialog.backspace();
    assert_eq!(dialog.name, "w");
    for field in [SessionField::Host, SessionField::Port, SessionField::User] {
        tab_to(&mut dialog, field);
        dialog.push('9');
    }
    assert_eq!(dialog.host, "9");
    assert_eq!(dialog.port, "229");
    assert_eq!(dialog.user, "9");
    tab_to(&mut dialog, SessionField::RemoteDir);
    dialog.push('/');
    assert_eq!(dialog.remote_dir, "/");
}

#[test]
fn a_choice_or_a_button_takes_no_text() {
    let mut dialog = SessionEdit::create();
    for field in [SessionField::Protocol, SessionField::Ok] {
        tab_to(&mut dialog, field);
        dialog.push('x');
        dialog.backspace();
    }
    let mut untouched = SessionEdit::create();
    tab_to(&mut untouched, SessionField::Ok);
    assert_eq!(dialog, untouched);
}

#[test]
fn the_protocol_cycles_both_ways_and_the_port_follows_its_default() {
    let mut dialog = SessionEdit::create();
    dialog.cycle_choice(true);
    assert_eq!(dialog.protocol, Protocol::Sftp, "the name is not a choice");
    tab_to(&mut dialog, SessionField::Protocol);
    dialog.cycle_choice(true);
    assert_eq!(dialog.protocol, Protocol::Scp);
    assert_eq!(dialog.port, "22", "SCP shares SFTP's port");
    dialog.cycle_choice(true);
    assert_eq!(dialog.protocol, Protocol::Ftp);
    assert_eq!(dialog.port, "21");
    dialog.port = "2121".to_owned();
    dialog.cycle_choice(false);
    assert_eq!(dialog.protocol, Protocol::Scp);
    assert_eq!(dialog.port, "2121", "a port typed by hand stays");
}

#[test]
fn the_login_kind_cycles_both_ways() {
    let mut dialog = SessionEdit::create();
    tab_to(&mut dialog, SessionField::Auth);
    dialog.cycle_choice(true);
    assert_eq!(dialog.auth, AuthKind::KeyFile);
    dialog.cycle_choice(false);
    dialog.cycle_choice(false);
    assert_eq!(dialog.auth, AuthKind::Agent);
}
