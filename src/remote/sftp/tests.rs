//! SFTP metadata and protocol error translation without a live SSH server.

use super::{at, fingerprint, metadata};
use crate::models::{EntryKind, FileSize, Modified, RemotePath};
use crate::remote::{RemoteError, UnixMode};
use std::{
    io,
    net::TcpListener,
    thread,
    time::{Duration, Instant, UNIX_EPOCH},
};

#[test]
fn a_closed_ssh_handshake_is_reported_without_exposing_protocol_details() {
    use crate::models::{HostName, Port, Protocol, Session};
    use crate::remote::{connect_with, Credentials};
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("loopback listener");
    listener
        .set_nonblocking(true)
        .expect("nonblocking listener");
    let port = listener.local_addr().expect("address").port();
    let server = thread::spawn(move || close_one_connection(&listener));
    let session = Session::new(
        Protocol::Sftp,
        HostName::new("127.0.0.1").expect("host"),
        Port::new(port).expect("port"),
        None,
    );
    let credentials = Credentials {
        user: "fixture".to_owned(),
        ..Credentials::default()
    };
    assert!(matches!(
        connect_with(&session, &credentials),
        Err(RemoteError::Failed("SSH handshake failed"))
    ));
    server.join().expect("server thread");
}

fn close_one_connection(listener: &TcpListener) {
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        match listener.accept() {
            Ok((stream, _)) => {
                drop(stream);
                return;
            }
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                assert!(Instant::now() < deadline, "client never connected");
                thread::sleep(Duration::from_millis(5));
            }
            Err(error) => panic!("accept failed: {error}"),
        }
    }
}

#[test]
fn sftp_status_codes_preserve_actionable_errors() {
    let path = RemotePath::parse("/files/report.txt").expect("remote path");
    for (code, expected) in [
        (2, RemoteError::NotFound(path.clone())),
        (
            3,
            RemoteError::Failed("remote operation: permission denied"),
        ),
        (11, RemoteError::AlreadyExists(path.clone())),
        (4, RemoteError::Failed("SFTP operation failed")),
    ] {
        let error = ssh2::Error::from_errno(ssh2::ErrorCode::SFTP(code));
        assert_eq!(at(&error, &path), expected);
    }
}

#[test]
fn host_fingerprint_uses_unpadded_sha256_base64() {
    assert_eq!(
        fingerprint(b"abc"),
        "SHA256:ungWv48Bz+pBQUDeXa4iI7ADYaOWF3qctBD/YfIAFa0"
    );
}

#[test]
fn metadata_preserves_remote_file_facts_and_masks_mode() {
    let stat = ssh2::FileStat {
        size: Some(42),
        uid: Some(1001),
        gid: None,
        perm: Some(0o100_755),
        atime: None,
        mtime: Some(60),
    };
    let facts = metadata::metadata(&stat);
    assert_eq!(facts.kind, EntryKind::File);
    assert_eq!(facts.size, FileSize::from_bytes(42));
    assert_eq!(
        facts.modified,
        Modified::new(UNIX_EPOCH + Duration::from_secs(60))
    );
    assert_eq!(facts.mode, Some(UnixMode::from_mode(0o755)));

    let entry = metadata::entry("script.sh".into(), &stat);
    assert_eq!(entry.name, "script.sh");
    assert_eq!(entry.size, facts.size);
    assert!(entry.executable);
    assert_eq!(entry.owner.as_deref(), Some("1001"));
}

#[test]
fn metadata_distinguishes_directories_links_and_missing_facts() {
    let mut stat = metadata::empty_stat();
    let unknown = metadata::metadata(&stat);
    assert_eq!(unknown.kind, EntryKind::File);
    assert_eq!(unknown.size, FileSize::from_bytes(0));
    assert_eq!(unknown.modified, Modified::UNKNOWN);
    assert_eq!(unknown.mode, None);
    assert!(!metadata::entry("unknown".into(), &stat).executable);
    stat.perm = Some(0o040_700);
    assert_eq!(metadata::metadata(&stat).kind, EntryKind::Directory);
    stat.perm = Some(0o120_777);
    assert_eq!(metadata::metadata(&stat).kind, EntryKind::SymlinkToFile);
}
