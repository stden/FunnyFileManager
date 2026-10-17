//! Unit tests: a session names itself `NetBox`'s way, describes its address,
//! and round-trips through JSON without ever carrying a password.

use std::path::PathBuf;

use super::{Auth, AuthKind, HostName, KeyFile, Origin, Port, Protocol, Session, UserName};
use crate::models::RemotePath;

fn session(protocol: Protocol, port: u16, user: Option<&str>) -> Session {
    Session::new(
        protocol,
        HostName::new("example.test").expect("host"),
        Port::new(port).expect("port"),
        user.map(|user| UserName::new(user).expect("user")),
    )
}

#[test]
fn protocols_know_their_ports_labels_and_schemes() {
    let table = [
        (Protocol::Sftp, 22, "SFTP", "sftp"),
        (Protocol::Scp, 22, "SCP", "scp"),
        (Protocol::Ftp, 21, "FTP", "ftp"),
        (Protocol::Ftps, 21, "FTPS", "ftps"),
    ];
    for (protocol, port, label, scheme) in table {
        assert_eq!(protocol.default_port().get(), port);
        assert_eq!(protocol.label(), label);
        assert_eq!(protocol.scheme(), scheme);
    }
    assert_eq!(Protocol::ALL.len(), table.len());
    assert_eq!(Protocol::default(), Protocol::Sftp);
}

#[test]
fn a_new_session_is_named_after_its_login_and_logs_in_by_password() {
    let plain = session(Protocol::Sftp, 22, Some("demo"));
    assert_eq!(plain.name.as_str(), "demo@example.test");
    assert_eq!(plain.auth, Auth::Password);
    assert!(plain.origin.is_manual());
    assert_eq!(plain.remote_dir, None);
    assert_eq!(plain.login(), "demo@example.test");

    let ftp = session(Protocol::Ftp, 2121, None);
    assert_eq!(ftp.name.as_str(), "example.test:2121");
    assert_eq!(ftp.login(), "example.test");
}

#[test]
fn the_url_writes_the_port_only_off_the_default_and_the_directory_after_it() {
    let mut on_default = session(Protocol::Scp, 22, Some("demo"));
    assert_eq!(on_default.url(), "scp://demo@example.test");
    on_default.remote_dir = Some(RemotePath::parse("/var/www").expect("dir"));
    assert_eq!(on_default.url(), "scp://demo@example.test/var/www");
    let elsewhere = session(Protocol::Ftps, 990, None);
    assert_eq!(elsewhere.url(), "ftps://example.test:990");
}

#[test]
fn an_auth_names_its_kind_and_the_kinds_their_labels() {
    let key = KeyFile::new("/home/bob/.ssh/id_ed25519").expect("key file");
    assert_eq!(Auth::Password.kind(), AuthKind::Password);
    assert_eq!(Auth::KeyFile(key).kind(), AuthKind::KeyFile);
    assert_eq!(Auth::Agent.kind(), AuthKind::Agent);
    let labels: Vec<&str> = AuthKind::ALL.iter().map(|kind| kind.label()).collect();
    assert_eq!(labels, ["Password", "Key file", "Agent"]);
    assert!(!Origin::NetBoxFile(PathBuf::from("/srv/x/demo.netbox")).is_manual());
}

#[test]
fn a_session_round_trips_through_json_with_no_password_field() {
    let mut original = session(Protocol::Sftp, 2222, Some("demo"));
    original.auth = Auth::KeyFile(KeyFile::new("/home/bob/.ssh/id").expect("key"));
    original.remote_dir = Some(RemotePath::parse("/var/www").expect("dir"));
    original.origin = Origin::NetBoxFile(PathBuf::from("/srv/x/demo.netbox"));

    let text = serde_json::to_string(&original).expect("serialise");
    assert!(!text.to_lowercase().contains("password"), "{text}");
    let back: Session = serde_json::from_str(&text).expect("parse");
    assert_eq!(back, original);
}

#[test]
fn optional_settings_may_be_left_out_of_the_file() {
    let back: Session =
        serde_json::from_str(r#"{"name":"x","protocol":"Ftp","host":"example.test","port":21}"#)
            .expect("parse");
    assert_eq!(back.user, None);
    assert_eq!(back.auth, Auth::Password);
    assert_eq!(back.remote_dir, None);
    assert!(back.origin.is_manual());
    let text = serde_json::to_string(&back).expect("serialise");
    assert!(
        !text.contains("user") && !text.contains("remote_dir"),
        "{text}"
    );
}

#[test]
fn a_file_value_that_fails_its_check_does_not_load() {
    let bad_host = r#"{"name":"x","protocol":"Sftp","host":"a b","port":22}"#;
    assert!(serde_json::from_str::<Session>(bad_host).is_err());
    let bad_port = r#"{"name":"x","protocol":"Sftp","host":"h","port":0}"#;
    assert!(serde_json::from_str::<Session>(bad_port).is_err());
    let missing_host = r#"{"name":"x","protocol":"Sftp","port":22}"#;
    assert!(serde_json::from_str::<Session>(missing_host).is_err());
}
