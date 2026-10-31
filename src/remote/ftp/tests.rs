//! Purpose: Real FTP adapter against a loopback server; no external hosts or credentials.

mod server;

use crate::models::{HostName, Port, Protocol, RemotePath, Session};
use crate::remote::{connect_with, Credentials, RemoteError, UnixMode, WriteMode};

fn path(text: &str) -> RemotePath {
    RemotePath::parse(text).expect("path")
}

#[test]
fn native_ftp_streams_renames_removes_and_changes_linux_permissions() {
    let server = server::Server::start(false);
    let mut fs = connection(&server);
    let (first, second) = (path("/a file.txt"), path("/renamed.txt"));
    fs.upload(
        &first,
        &mut b"binary\0\xff\n".as_slice(),
        WriteMode::CreateNew,
    )
    .expect("upload");
    assert!(matches!(
        fs.upload(&first, &mut b"bad".as_slice(), WriteMode::CreateNew),
        Err(RemoteError::AlreadyExists(_))
    ));
    let mut bytes = Vec::new();
    fs.download(&first, &mut bytes).expect("download");
    assert_eq!(bytes, b"binary\0\xff\n");
    fs.chmod(&first, UnixMode::parse("0640").expect("mode"))
        .expect("chmod");
    fs.rename(&first, &second).expect("rename");
    assert_eq!(
        fs.list(&RemotePath::root()).expect("list")[0].name,
        "renamed.txt"
    );
    fs.remove_file(&second).expect("delete");
    assert_eq!(fs.list(&RemotePath::root()).expect("empty").len(), 0);
    drop(fs);
    assert!(server
        .finish()
        .iter()
        .any(|command| command == "SITE CHMOD 0640 /a file.txt"));
}

#[test]
fn ftp_rejects_command_injection_before_sending_a_path() {
    assert!(super::argument(&RemotePath::root().join("bad\r\nDELE other").expect("name")).is_err());
    assert_eq!(
        super::listing::parse(
            &["-rw-r----- 1 1000 1000 3 Nov 5 2025 a file.txt".to_owned()],
            false
        )
        .expect("Unix LIST")[0]
            .name,
        "a file.txt"
    );
}

fn connection(server: &server::Server) -> Box<dyn crate::remote::RemoteFs> {
    let session = Session::new(
        Protocol::Ftp,
        HostName::new("127.0.0.1").expect("host"),
        Port::new(server.port()).expect("port"),
        None,
    );
    let credentials = Credentials {
        user: "tester".to_owned(),
        password: "test-password".to_owned().into(),
        ..Credentials::default()
    };
    connect_with(&session, &credentials).expect("connect")
}
