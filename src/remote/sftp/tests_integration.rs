//! Real SFTP protocol on a private loopback sshd, enabled explicitly for CI.

mod fixture;

use std::{env, process::Command};

use crate::models::{DirEntry, EntryKind, RemotePath};
use crate::remote::{connect_with, Credentials, RemoteError, RemoteFs, UnixMode, WriteMode};

const ENABLE: &str = "FFM_TEST_SFTP_SERVER";
const CHILD: &str = "FFM_SFTP_FIXTURE_CHILD";
const TEST_NAME: &str = "remote::sftp::tests_integration::real_sftp_round_trip";
const SUCCESS: &str = "FFM_SFTP_FIXTURE_COMPLETED";

/// Run in a separate test process so HOME can be isolated without affecting
/// other tests or reading the developer's `known_hosts` file.
#[test]
fn real_sftp_round_trip() {
    if env::var(ENABLE).as_deref() != Ok("1") {
        return;
    }
    if env::var(CHILD).as_deref() == Ok("1") {
        run_round_trip();
        println!("{SUCCESS}");
        return;
    }
    let home = tempfile::tempdir().expect("isolated SSH home");
    let output = Command::new(env::current_exe().expect("test executable"))
        .args(["--exact", TEST_NAME, "--nocapture"])
        .env("HOME", home.path())
        .env(CHILD, "1")
        .output()
        .expect("isolated SFTP test");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "isolated SFTP test failed: {stdout}\n{stderr}"
    );
    assert!(
        stdout.contains(SUCCESS),
        "isolated SFTP test did not run: {stdout}\n{stderr}"
    );
}

fn run_round_trip() {
    let server = fixture::Server::start();
    let session = server.session();
    let mut credentials = Credentials::default();
    let fingerprint = match connect_with(&session, &credentials) {
        Err(RemoteError::HostKey(fingerprint)) => fingerprint,
        Err(error) => panic!("expected an unknown host key: {error}"),
        Ok(_) => panic!("an unknown host must require confirmation"),
    };
    assert_eq!(
        fingerprint,
        server.fingerprint(),
        "unexpected SFTP server host key"
    );
    credentials.trusted_host_key = Some("SHA256:wrong".to_owned());
    assert!(matches!(
        connect_with(&session, &credentials),
        Err(RemoteError::HostKey(_))
    ));
    credentials.trusted_host_key = Some(fingerprint);
    let mut remote = connect_with(&session, &credentials).unwrap_or_else(|error| {
        panic!(
            "trusted key authentication failed: {error}; sshd: {}",
            server.diagnostics()
        )
    });
    exercise_filesystem(remote.as_mut(), &server);
}

fn exercise_filesystem(remote: &mut dyn RemoteFs, server: &fixture::Server) {
    let directory = server.path("work");
    let source = server.path("work/source.txt");
    let renamed = server.path("work/renamed.txt");
    remote.mkdir(&directory).expect("mkdir");
    assert_eq!(
        remote.stat(&directory).expect("directory stat").kind,
        EntryKind::Directory
    );
    assert!(matches!(
        remote.stat(&source),
        Err(RemoteError::NotFound(_))
    ));
    let bytes = b"SFTP\0binary\xff";
    upload_new(remote, &source, bytes);
    remote.rename(&source, &renamed).expect("rename");
    check_download(remote, &renamed, bytes);
    check_replace(remote, server, &renamed, bytes);
    check_links(remote, server, &directory, &renamed);
    remote.remove_file(&renamed).expect("remove file");
    remote.rmdir(&directory).expect("remove directory");
}

fn upload_new(remote: &mut dyn RemoteFs, source: &RemotePath, bytes: &[u8]) {
    let mut input = bytes;
    assert_eq!(
        remote.upload(source, &mut input, WriteMode::CreateNew),
        Ok(u64::try_from(bytes.len()).expect("fixture size"))
    );
    assert!(remote
        .upload(source, &mut b"no".as_slice(), WriteMode::CreateNew)
        .is_err());
    assert!(matches!(
        remote.rename(source, source),
        Err(RemoteError::AlreadyExists(_))
    ));
}

fn check_download(remote: &mut dyn RemoteFs, path: &RemotePath, expected: &[u8]) {
    let mode = UnixMode::from_mode(0o640);
    remote.chmod(path, mode).expect("chmod");
    let facts = remote.stat(path).expect("file stat");
    assert_eq!(facts.kind, EntryKind::File);
    assert_eq!(facts.mode, Some(mode));
    let mut bytes = Vec::new();
    assert_eq!(
        remote.download(path, &mut bytes),
        Ok(u64::try_from(expected.len()).expect("fixture size"))
    );
    assert_eq!(bytes, expected);
}

fn check_replace(
    remote: &mut dyn RemoteFs,
    server: &fixture::Server,
    target: &RemotePath,
    original: &[u8],
) {
    let temporary = server.path("work/replacement");
    remote
        .upload(&temporary, &mut b"new".as_slice(), WriteMode::CreateNew)
        .expect("stage replacement");
    let replaced = remote.replace_file(&temporary, target).is_ok();
    let mut contents = Vec::new();
    remote.download(target, &mut contents).expect("reopen");
    if replaced {
        assert_eq!(contents, b"new");
        assert!(matches!(
            remote.stat(&temporary),
            Err(RemoteError::NotFound(_))
        ));
    } else {
        assert_eq!(contents, original);
        contents.clear();
        remote
            .download(&temporary, &mut contents)
            .expect("staged file");
        assert_eq!(contents, b"new");
        remote.remove_file(&temporary).expect("remove staging file");
    }
    remote
        .upload(target, &mut b"updated".as_slice(), WriteMode::Overwrite)
        .expect("overwrite existing file");
    contents.clear();
    remote.download(target, &mut contents).expect("reopen");
    assert_eq!(contents, b"updated");
}

fn check_links(
    remote: &mut dyn RemoteFs,
    server: &fixture::Server,
    directory: &RemotePath,
    target: &RemotePath,
) {
    use std::os::unix::fs::symlink;

    let file_link = server.local("work/link");
    let broken = server.local("work/broken");
    let directory_link = server.local("work/directory-link");
    symlink(target.as_str(), &file_link).expect("file symlink");
    symlink(server.local("missing"), &broken).expect("dangling symlink");
    symlink(server.local("work"), &directory_link).expect("directory symlink");
    let entries = remote.list(directory).expect("directory listing");
    assert_entry(&entries, "renamed.txt", EntryKind::File);
    assert_entry(&entries, "link", EntryKind::SymlinkToFile);
    assert_entry(&entries, "broken", EntryKind::BrokenSymlink);
    assert_entry(&entries, "directory-link", EntryKind::SymlinkToDirectory);
    for name in ["link", "broken", "directory-link"] {
        remote
            .remove_file(&server.path(&format!("work/{name}")))
            .expect("remove link");
    }
}

fn assert_entry(entries: &[DirEntry], name: &str, kind: EntryKind) {
    assert!(
        entries
            .iter()
            .any(|entry| entry.name == name && entry.kind == kind),
        "missing {name}"
    );
}
