//! FTP fallback and errors over a private loopback fixture.

use super::{connection, path, server};
use crate::models::{EntryKind, HostName, Port, Protocol, RemotePath, Session, UserName};
use crate::remote::{connect_with, Credentials, RemoteError, WriteMode};
use std::io::{self, Write};
use suppaftp::{types::Response, FtpError, Status};

#[test]
fn legacy_server_falls_back_to_list_and_preserves_file_names() {
    let server = server::Server::start(true);
    let mut fs = connection(&server);
    let source = path("/a file.txt");
    fs.upload(&source, &mut b"hello".as_slice(), WriteMode::Overwrite)
        .expect("upload");
    let entries = fs.list(&RemotePath::root()).expect("legacy list");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].name, "a file.txt");
    assert_eq!(entries[0].kind, EntryKind::File);
    drop(fs);
    let commands = server.finish();
    assert!(commands.iter().any(|command| command.starts_with("MLSD")));
    assert!(commands.iter().any(|command| command.starts_with("LIST")));
}

#[test]
fn ftp_replace_and_overwrite_preserve_exact_bytes() {
    let server = server::Server::start(false);
    let mut fs = connection(&server);
    let temporary = path("/temporary.txt");
    let target = path("/target.txt");
    fs.upload(&target, &mut b"old".as_slice(), WriteMode::Overwrite)
        .expect("old file");
    fs.upload(&temporary, &mut b"new".as_slice(), WriteMode::Overwrite)
        .expect("temporary");
    assert_eq!(
        fs.rename(&temporary, &target),
        Err(RemoteError::AlreadyExists(target.clone()))
    );
    fs.replace_file(&temporary, &target).expect("replace");
    let mut bytes = Vec::new();
    assert_eq!(fs.download(&target, &mut bytes), Ok(3));
    assert_eq!(bytes, b"new");
    drop(fs);
    let commands = server.finish();
    assert!(commands
        .iter()
        .any(|command| command == "RNFR /temporary.txt"));
    assert!(commands.iter().any(|command| command == "RNTO /target.txt"));
}

#[test]
fn refused_directory_operations_return_a_sanitized_error() {
    let server = server::Server::start(false);
    let mut fs = connection(&server);
    let target = path("/directory");
    let expected = Err(RemoteError::Failed("FTP server refused the operation"));
    assert_eq!(fs.mkdir(&target), expected);
    assert_eq!(fs.rmdir(&target), expected);
    drop(fs);
    let commands = server.finish();
    assert!(commands.iter().any(|command| command == "MKD /directory"));
    assert!(commands.iter().any(|command| command == "RMD /directory"));
}

#[test]
fn unsupported_reply_codes_and_transport_failures_are_distinguished() {
    for status in [
        Status::BadCommand,
        Status::BadArguments,
        Status::NotImplemented,
        Status::NotImplementedParameter,
    ] {
        let error = FtpError::UnexpectedResponse(Response {
            status,
            body: Vec::new(),
        });
        assert!(super::super::unsupported(&error));
        assert_eq!(
            super::super::failed(error),
            RemoteError::Failed("FTP server refused the operation")
        );
    }
    let error = FtpError::UnexpectedResponse(Response {
        status: Status::FileUnavailable,
        body: Vec::new(),
    });
    assert!(!super::super::unsupported(&error));
    let error = FtpError::ConnectionError(io::Error::from(io::ErrorKind::TimedOut));
    assert_eq!(
        super::super::failed(error),
        RemoteError::Failed("remote operation timed out")
    );
}

#[test]
fn bad_listings_are_rejected_and_missing_machine_permissions_are_not_guessed() {
    assert!(super::super::listing::parse(&["type=unknown; file".into()], true).is_err());
    assert!(super::super::listing::parse(&["type=unknown; file".into()], false).is_err());
    assert!(
        super::super::listing::parse(&["type=file;unix.mode=garbage; x".into()], true).is_err()
    );
    let listing = super::super::listing::parse(&["type=file;size=3; x".into()], true)
        .expect("valid entry without a mode");
    assert_eq!(listing[0].name, "x");
    assert_eq!(listing[0].attributes.bits(), 0);
}

#[test]
fn credentials_user_overrides_session_user_and_empty_credentials_fall_back() {
    for (supplied, expected) in [("", "saved"), ("other", "other")] {
        let server = server::Server::start(false);
        let session = Session::new(
            Protocol::Ftp,
            HostName::new("127.0.0.1").expect("host"),
            Port::new(server.port()).expect("port"),
            Some(UserName::new("saved").expect("user")),
        );
        let credentials = Credentials {
            user: supplied.to_owned(),
            ..Credentials::default()
        };
        let connection = connect_with(&session, &credentials).expect("login");
        drop(connection);
        assert!(server.finish().contains(&format!("USER {expected}")));
    }
}

struct RefusingWriter;

impl Write for RefusingWriter {
    fn write(&mut self, _bytes: &[u8]) -> io::Result<usize> {
        Err(io::Error::from(io::ErrorKind::PermissionDenied))
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn download_reports_local_write_failures_and_finishes_the_transfer() {
    let server = server::Server::start(false);
    let mut fs = connection(&server);
    let source = path("/file.txt");
    fs.upload(&source, &mut b"data".as_slice(), WriteMode::Overwrite)
        .expect("upload");
    assert_eq!(
        fs.download(&source, &mut RefusingWriter),
        Err(RemoteError::Failed("remote operation: permission denied"))
    );
    assert_eq!(
        fs.list(&RemotePath::root()).expect("subsequent list").len(),
        1
    );
    drop(fs);
    assert!(server
        .finish()
        .iter()
        .any(|command| command == "RETR /file.txt"));
}
