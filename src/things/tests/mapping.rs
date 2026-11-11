//! Machines become SFTP sessions on port 22 with a password asked for at
//! connection; the host falls back to the IP address; a bad name or host is
//! skipped with its reason; a repeated name is a duplicate.

use super::machine;
use crate::models::{Auth, Field, InvalidField, Origin, Port, Problem, Protocol, Session};
use crate::sessions::import::{Imported, SkipReason};
use crate::things::{sessions_from, Machine};

fn import(machines: &[Machine]) -> Imported {
    sessions_from(machines)
}

fn only_session(machines: &[Machine]) -> Session {
    let imported = import(machines);
    assert!(imported.skipped.is_empty(), "{:?}", imported.skipped);
    let [session] = imported.sessions.as_slice() else {
        panic!("one session, got {}", imported.sessions.len());
    };
    session.clone()
}

#[test]
fn the_hostname_is_the_host() {
    let mut web = machine("web-01", "web-01.example.com");
    web.ip_address = Some("10.0.0.5".to_owned());
    assert_eq!(only_session(&[web]).host.as_str(), "web-01.example.com");
}

#[test]
fn the_ip_address_is_the_host_when_the_hostname_is_blank() {
    let mut web = machine("web-01", "  ");
    web.ip_address = Some("10.0.0.5".to_owned());
    assert_eq!(only_session(&[web]).host.as_str(), "10.0.0.5");
}

#[test]
fn a_machine_with_no_host_is_skipped_with_the_reason() {
    let imported = import(&[machine("web-01", "")]);
    assert_eq!(imported.sessions, Vec::<Session>::new());
    assert_eq!(imported.skipped.len(), 1);
    assert_eq!(
        imported.skipped[0].reason,
        SkipReason::Invalid(InvalidField::new(Field::Host, Problem::Empty))
    );
    assert_eq!(imported.skipped[0].name(), "web-01");
}

#[test]
fn a_machine_with_no_name_is_skipped_under_a_placeholder() {
    let imported = import(&[machine("", "web-01.example.com")]);
    assert_eq!(imported.sessions, Vec::<Session>::new());
    assert_eq!(
        imported.skipped[0].reason,
        SkipReason::Invalid(InvalidField::new(Field::Name, Problem::Empty))
    );
    assert_eq!(imported.skipped[0].name(), "(no name)");
}

#[test]
fn the_session_is_named_for_the_machine_and_keeps_its_id() {
    let session = only_session(&[machine("web-01", "web-01.example.com")]);
    assert_eq!(session.name.as_str(), "web-01");
    assert_eq!(
        session.origin,
        Origin::Things {
            machine: "id-web-01".to_owned()
        }
    );
}

#[test]
fn the_session_is_sftp_on_port_22_with_password_login_and_no_user_or_directory() {
    let session = only_session(&[machine("web-01", "web-01.example.com")]);
    assert_eq!(session.protocol, Protocol::Sftp);
    assert_eq!(session.port, Port::SSH);
    assert_eq!(session.user, None);
    assert_eq!(session.auth, Auth::Password);
    assert_eq!(session.remote_dir, None);
}

#[test]
fn a_name_an_earlier_machine_took_is_a_duplicate() {
    let imported = import(&[
        machine("web-01", "a.example.com"),
        machine("web-01", "b.example.com"),
    ]);
    assert_eq!(imported.sessions.len(), 1);
    assert_eq!(imported.sessions[0].host.as_str(), "a.example.com");
    assert_eq!(imported.skipped[0].reason, SkipReason::Duplicate);
}
