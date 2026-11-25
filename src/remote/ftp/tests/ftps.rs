//! Purpose: Loopback anonymous FTP and explicit/implicit FTPS tests.

use super::{ftps_fixture, server};
use crate::models::{HostName, Port, Protocol, RemotePath, Session};
use crate::remote::ftp::tls::{
    build_tls_config, connect_with_config, default_tls_config, tls_failed,
};
use crate::remote::{connect_with, Credentials, RemoteError};

fn loopback(protocol: Protocol, port: u16) -> Session {
    Session::new(
        protocol,
        HostName::new("127.0.0.1").expect("host"),
        Port::new(port).expect("port"),
        None,
    )
}

#[test]
fn anonymous_ftp_defaults_user_and_password_and_ignores_blank_listing_lines() {
    let server = server::Server::start(false);
    let fs = connect_with(
        &loopback(Protocol::Ftp, server.port()),
        &Credentials::default(),
    )
    .expect("anonymous connect");
    drop(fs);
    let commands = server.finish();
    assert!(commands.contains(&"USER anonymous".to_owned()));
    assert!(commands.contains(&"PASS anonymous@".to_owned()));

    let parsed = super::super::listing::parse(
        &[String::new(), "   ".to_owned(), "total 0".to_owned()],
        false,
    )
    .expect("blank lines ignored");
    assert_eq!(parsed, []);
}

#[test]
fn explicit_and_implicit_ftps_connect_and_list_over_tls() {
    for implicit in [false, true] {
        let server = ftps_fixture::TlsFtpServer::start(implicit, false);
        let session = loopback(Protocol::Ftps, server.port);
        let mut fs = connect_with_config(
            &session,
            &Credentials::default(),
            "anonymous",
            implicit,
            ftps_fixture::client_config(),
        )
        .expect("ftps connect");
        let entries = fs.list(&RemotePath::root()).expect("mlsd over tls");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "tls.txt");
        drop(fs);
        let commands = server.finish();
        assert_eq!(commands.contains(&"AUTH TLS".to_owned()), !implicit);
    }
}

#[test]
fn ftps_rejects_untrusted_certificates_and_refused_tls() {
    let server = ftps_fixture::TlsFtpServer::start(false, false);
    let untrusted = connect_with(
        &loopback(Protocol::Ftps, server.port),
        &Credentials::default(),
    );
    assert!(matches!(
        untrusted,
        Err(RemoteError::Failed("FTPS TLS handshake failed"))
    ));
    drop(server.finish());

    let refused = ftps_fixture::TlsFtpServer::start(false, true);
    let denied = connect_with_config(
        &loopback(Protocol::Ftps, refused.port),
        &Credentials::default(),
        "anonymous",
        false,
        default_tls_config().expect("default config"),
    );
    assert!(matches!(denied, Err(RemoteError::Failed(_))));
    drop(refused.finish());
}

#[test]
fn ftps_maps_closed_ports_and_tls_configuration_failures() {
    assert_eq!(
        build_tls_config(&[]).err(),
        Some(RemoteError::Failed("FTPS TLS configuration failed"))
    );
    assert_eq!(
        tls_failed(suppaftp::FtpError::SecureError("invalid".to_owned())),
        RemoteError::Failed("FTPS TLS handshake failed")
    );

    let closed_listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    let closed_port = closed_listener.local_addr().expect("addr").port();
    drop(closed_listener);
    let closed_implicit = connect_with_config(
        &loopback(Protocol::Ftps, closed_port),
        &Credentials::default(),
        "anonymous",
        true,
        ftps_fixture::client_config(),
    );
    assert!(matches!(closed_implicit, Err(RemoteError::Failed(_))));
}

#[test]
fn ftp_maps_login_rejection_to_sanitized_authentication_error() {
    let server = server::Server::start(false);
    let credentials = Credentials {
        user: "denied".to_owned(),
        ..Credentials::default()
    };
    let result = connect_with(&loopback(Protocol::Ftp, server.port()), &credentials);
    assert!(matches!(
        result,
        Err(RemoteError::Failed("FTP authentication failed"))
    ));
    drop(server.finish());
}
