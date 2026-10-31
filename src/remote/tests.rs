//! Unit tests: this build connects nowhere and says so per protocol, errors
//! read as one line, and a link is one shared connection that recovers from
//! a poisoned lock.

use super::fake::FakeRemoteFs;
use super::{connect, RemoteError, RemoteLink};
use crate::models::{HostName, Port, Protocol, RemotePath, Session};

mod contract;

fn session(protocol: Protocol) -> Session {
    Session::new(
        protocol,
        HostName::new("example.test").expect("host"),
        Port::SSH,
        None,
    )
}

#[test]
fn unsupported_protocols_remain_explicit_and_supported_ones_require_credentials() {
    for protocol in [Protocol::Scp, Protocol::Ftps] {
        let error = connect(&session(protocol))
            .map(drop)
            .expect_err("no transport");
        assert_eq!(error, RemoteError::NoTransport(protocol));
        assert!(error.to_string().contains(protocol.label()), "{error}");
    }
    for protocol in [Protocol::Sftp, Protocol::Ftp] {
        let error = connect(&session(protocol))
            .map(drop)
            .expect_err("credentials required");
        assert_eq!(error, RemoteError::AuthenticationRequired);
    }
    assert_eq!(
        RemoteError::NoTransport(Protocol::Sftp).to_string(),
        "this build cannot connect over SFTP; the session is stored for when it can"
    );
}

#[test]
fn listing_errors_name_the_directory() {
    let path = RemotePath::parse("/var/www").expect("path");
    assert_eq!(
        RemoteError::NotFound(path.clone()).to_string(),
        "no such remote directory: /var/www"
    );
    assert_eq!(
        RemoteError::NotADirectory(path).to_string(),
        "not a directory: /var/www"
    );
}

#[test]
fn a_link_is_one_connection_and_survives_a_panic_while_it_is_held() {
    let link = RemoteLink::new(Box::new(FakeRemoteFs::with_files(&[("/a/b.txt", 3)])));
    let shared = link.clone();
    let poisoner = shared.clone();
    let panicked = std::thread::spawn(move || {
        let _held = poisoner.lock();
        panic!("a listing that panicked while holding the connection");
    })
    .join();
    assert!(panicked.is_err());
    let listing = link
        .lock()
        .list(&RemotePath::root())
        .expect("the connection still answers");
    assert_eq!(listing.len(), 1);
    assert_eq!(link, shared);
}
