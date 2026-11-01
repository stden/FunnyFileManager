//! Purpose: A `<Session>` becomes a session with every value read the way
//! `NetBox` writes it, and a saved password is counted, never read.

use std::fs;

use tempfile::TempDir;

use super::super::import;
use super::{only_session, only_skip, parsed, sessions, FILE};
use crate::models::{Auth, KeyFile, Origin, Protocol, RemotePath, UserName};
use crate::sessions::import::{Imported, SkipReason};

/// `NetBox`'s export of one made-up session. The password is 100 made-up hex
/// digits: the importer counts it and never decodes it.
const EXAMPLE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<NetBox version="2.1">
  <Sessions>
    <Session name="demo@example.test">
      <HostName>example.test</HostName>
      <PortNumber>2222</PortNumber>
      <UserName>demo</UserName>
      <FSProtocol>SFTP%20(SCP)</FSProtocol>
      <RemoteDirectory>%2Fvar%2Fwww</RemoteDirectory>
      <Password>A35C4D5E6F7A8B9CA35C4D5E6F7A8B9CA35C4D5E6F7A8B9CA35C4D5E6F7A8B9CA35C4D5E6F7A8B9CA35C4D5E6F7A8B9CA35C</Password>
    </Session>
  </Sessions>
</NetBox>
"#;

#[test]
fn the_example_export_becomes_one_session() {
    let imported = parsed(EXAMPLE);
    assert_eq!(imported.passwords_left_out, 1);
    let [session] = imported.sessions.as_slice() else {
        panic!("one session");
    };
    assert_eq!(session.name.as_str(), "demo@example.test");
    assert_eq!(session.host.as_str(), "example.test");
    assert_eq!(session.port.get(), 2222);
    assert_eq!(session.user.as_ref().map(UserName::as_str), Some("demo"));
    assert_eq!(session.protocol, Protocol::Sftp);
    assert_eq!(session.remote_dir, RemotePath::parse("/var/www").ok());
    assert_eq!(session.auth, Auth::Password);
    assert_eq!(session.origin, Origin::NetBoxFile { path: FILE.into() });
    assert!(
        !format!("{imported:?}").contains("A35C4D"),
        "the password is not carried"
    );
}

#[test]
fn import_reads_the_file_it_is_given() {
    let dir = TempDir::new().expect("temp dir");
    let file = dir.path().join("demo.netbox");
    fs::write(&file, EXAMPLE).expect("write");
    let imported = import(&file).expect("imported");
    assert_eq!(imported.sessions.len(), 1);
    assert_eq!(
        imported.sessions[0].origin,
        Origin::NetBoxFile { path: file }
    );
}

#[test]
fn a_session_without_a_name_is_named_netbox_s_way() {
    let session = only_session(&sessions(
        "<Session><HostName>h.test</HostName><UserName>u</UserName><PortNumber>2200</PortNumber></Session>",
    ));
    assert_eq!(session.name.as_str(), "u@h.test:2200");
    let bare = only_session(&sessions("<Session><HostName>h.test</HostName></Session>"));
    assert_eq!(bare.name.as_str(), "h.test");
    assert_eq!(bare.port.get(), 22, "SFTP's port when none is given");
    assert_eq!(bare.user, None);
}

#[test]
fn blank_optional_settings_are_not_set() {
    let session = only_session(&sessions(
        "<Session><HostName>h.test</HostName><UserName> </UserName><RemoteDirectory/><PublicKeyFile></PublicKeyFile></Session>",
    ));
    assert_eq!(session.user, None);
    assert_eq!(session.remote_dir, None);
    assert_eq!(session.auth, Auth::Password);
}

#[test]
fn a_public_key_file_logs_in_by_key() {
    let session = only_session(&sessions(
        "<Session><HostName>h.test</HostName><PublicKeyFile>%2Fhome%2Fbob%2F.ssh%2Fid</PublicKeyFile></Session>",
    ));
    assert_eq!(
        session.auth,
        Auth::KeyFile(KeyFile::new("/home/bob/.ssh/id").expect("key"))
    );
}

#[test]
fn protocols_are_read_by_number_or_by_name() {
    let table = [
        ("0", "", Protocol::Scp),
        ("1", "", Protocol::Sftp),
        ("2", "", Protocol::Sftp),
        ("5", "", Protocol::Ftp),
        ("5", "0", Protocol::Ftp),
        ("5", "3", Protocol::Ftps),
        ("ftp", "1", Protocol::Ftps),
        ("scp", "1", Protocol::Scp),
        ("SFTP", "", Protocol::Sftp),
        (" sftp%20(scp) ", "", Protocol::Sftp),
        ("FTPS", "", Protocol::Ftps),
        ("", "", Protocol::Sftp),
    ];
    for (fs_protocol, ftps, expected) in table {
        let session = only_session(&sessions(&format!(
            "<Session><HostName>h.test</HostName><FSProtocol>{fs_protocol}</FSProtocol><Ftps>{ftps}</Ftps></Session>"
        )));
        assert_eq!(session.protocol, expected, "{fs_protocol:?} / {ftps:?}");
    }
    for unsupported in ["6", "7", "WebDAV", "S3"] {
        let skipped = only_skip(&sessions(&format!(
            "<Session name=\"w\"><HostName>h.test</HostName><FSProtocol>{unsupported}</FSProtocol></Session>"
        )));
        assert_eq!(
            skipped.reason,
            SkipReason::UnsupportedProtocol,
            "{unsupported}"
        );
    }
}

#[test]
fn passwords_are_counted_only_when_there_is_one() {
    let imported = parsed(&sessions(
        "<Session><HostName>a.test</HostName><Password>00</Password></Session>\
         <Session><HostName>b.test</HostName><Password> </Password></Session>\
         <Session><HostName>c;</HostName><Password>11</Password></Session>",
    ));
    assert_eq!(
        imported.passwords_left_out, 2,
        "a skipped entry's counts too"
    );
    assert_eq!(imported.sessions.len(), 2);
}

#[test]
fn sessions_are_read_from_every_sessions_list_and_nothing_else() {
    let imported = parsed(
        "<NetBox><Sessions><Session><HostName>a.test</HostName></Session><Other/></Sessions>\
         <Sessions><Session><HostName>b.test</HostName></Session></Sessions>\
         <Session><HostName>c.test</HostName></Session></NetBox>",
    );
    let hosts: Vec<&str> = imported.sessions.iter().map(|s| s.host.as_str()).collect();
    assert_eq!(hosts, ["a.test", "b.test"]);
    assert_eq!(parsed("<NetBox/>"), Imported::default());
}
