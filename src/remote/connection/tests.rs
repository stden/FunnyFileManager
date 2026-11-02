//! Purpose: Unit tests for loopback TCP socket setup, DNS failure, and credential/protocol guards.

use super::{connect_with, socket, Credentials};
use crate::models::{HostName, Port, Protocol, Session, UserName};
use crate::remote::RemoteError;
use std::net::TcpListener;

#[test]
fn socket_connects_to_loopback_and_rejects_unresolvable_or_closed_hosts() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = Port::new(listener.local_addr().unwrap().port()).unwrap();
    let handle = std::thread::spawn(move || drop(listener.accept()));
    let session = Session::new(
        Protocol::Sftp,
        HostName::new("127.0.0.1").unwrap(),
        port,
        Some(UserName::new("alice").unwrap()),
    );
    assert!(socket(&session).is_ok());
    handle.join().unwrap();

    let bad_dns = Session::new(
        Protocol::Sftp,
        HostName::new("unresolvable.invalid").unwrap(),
        Port::SSH,
        None,
    );
    assert!(matches!(socket(&bad_dns), Err(RemoteError::Failed(_))));
}

#[test]
fn connect_with_rejects_unsupported_protocol_and_missing_user() {
    let scp = Session::new(
        Protocol::Scp,
        HostName::new("127.0.0.1").unwrap(),
        Port::SSH,
        None,
    );
    assert!(matches!(
        connect_with(&scp, &Credentials::default()),
        Err(RemoteError::NoTransport(Protocol::Scp))
    ));
    let no_user = Session::new(
        Protocol::Sftp,
        HostName::new("127.0.0.1").unwrap(),
        Port::SSH,
        None,
    );
    assert!(matches!(
        connect_with(&no_user, &Credentials::default()),
        Err(RemoteError::AuthenticationRequired)
    ));
}

#[test]
fn connect_with_reports_error_on_closed_ftp_endpoint() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = Port::new(listener.local_addr().unwrap().port()).unwrap();
    drop(listener);
    let ftp = Session::new(
        Protocol::Ftp,
        HostName::new("127.0.0.1").unwrap(),
        port,
        None,
    );
    let creds = Credentials {
        user: "alice".into(),
        ..Credentials::default()
    };
    assert!(connect_with(&ftp, &creds).is_err());
}
