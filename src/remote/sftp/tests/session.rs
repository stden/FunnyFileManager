//! Cross-platform unit tests for `connect_over`, `authenticate`, `verify_host`, and `known_hosts`.

use super::super::{
    authenticate, connect_over,
    filesystem::tests_fake::{FakeBackend, FakeFile, FAKE_SFTP_OPS},
    fingerprint, known_hosts,
    ops::{ClientOps, SSH2_CLIENT_OPS},
    verify_host,
};
use crate::models::{Auth, HostName, KeyFile, Port, Protocol, Session};
use crate::remote::{Credentials, RemoteError};
use ssh2::{CheckResult, Error, ErrorCode, HostKeyType, KnownHostFileKind};
use std::{
    cell::Cell,
    fs,
    net::{TcpListener, TcpStream},
    path::Path,
};
use tempfile::TempDir;

struct FakeSession {
    key: Option<&'static [u8]>,
    check: CheckResult,
    read_fails: bool,
    authed: bool,
    sftp_fails: bool,
    seen_timeout: Cell<u32>,
}

struct FakeKnownHosts {
    check: CheckResult,
    read_fails: bool,
}

static FAKE_CLIENT_OPS: ClientOps<FakeSession, FakeKnownHosts, FakeBackend, FakeFile> = ClientOps {
    new_session: || {
        Ok(FakeSession {
            key: Some(b"host-key"),
            check: CheckResult::Match,
            read_fails: false,
            authed: true,
            sftp_fails: false,
            seen_timeout: Cell::new(0),
        })
    },
    set_timeout: |s, ms| s.seen_timeout.set(ms),
    set_tcp_stream: |_, _| {},
    handshake: |_| Ok(()),
    host_key: |s| s.key.map(|k| (k, HostKeyType::Ed25519)),
    known_hosts: |s| {
        Ok(FakeKnownHosts {
            check: s.check,
            read_fails: s.read_fails,
        })
    },
    read_known_hosts: |k, _: &Path, _: KnownHostFileKind| {
        if k.read_fails {
            Err(Error::new(ErrorCode::Session(-1), "bad known_hosts"))
        } else {
            Ok(1)
        }
    },
    check_port: |k, _, _, _| k.check,
    auth_password: |_, _, _| Ok(()),
    auth_pubkey_file: |_, _, _, _, _| Ok(()),
    auth_agent: |_, _| Ok(()),
    authenticated: |s| s.authed,
    open_sftp: |s| {
        if s.sftp_fails {
            Err(Error::new(ErrorCode::Session(-1), "no sftp"))
        } else {
            Ok(FakeBackend::default())
        }
    },
    sftp_ops: &FAKE_SFTP_OPS,
};

fn loopback_stream() -> TcpStream {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("listener");
    let addr = listener.local_addr().expect("addr");
    let stream = TcpStream::connect(addr).expect("connect");
    drop(listener.accept().expect("accept"));
    stream
}

fn fixture_session(auth: Auth) -> Session {
    let mut s = Session::new(
        Protocol::Sftp,
        HostName::new("127.0.0.1").expect("host"),
        Port::new(2222).expect("port"),
        None,
    );
    s.auth = auth;
    s
}

fn restore_env(name: &str, previous: Option<std::ffi::OsString>) {
    match previous {
        Some(v) => std::env::set_var(name, v),
        None => std::env::remove_var(name),
    }
}

#[test]
fn known_hosts_and_connect_over_cover_env_fallbacks_and_auth_modes() {
    let _lock = crate::test_support::env_lock();
    let home = TempDir::new().expect("temp home");
    let prev_home = std::env::var_os("HOME");
    let prev_profile = std::env::var_os("USERPROFILE");
    std::env::remove_var("HOME");
    std::env::set_var("USERPROFILE", home.path());
    assert_eq!(known_hosts(), Some(home.path().join(".ssh/known_hosts")));
    std::env::remove_var("USERPROFILE");
    assert_eq!(known_hosts(), None);
    let (ag, c) = (fixture_session(Auth::Agent), Credentials::default());
    assert!(connect_over(&FAKE_CLIENT_OPS, loopback_stream(), &ag, &c, "u").is_ok());
    fs::create_dir_all(home.path().join(".ssh")).expect("mkdir .ssh");
    fs::write(home.path().join(".ssh/known_hosts"), b"").expect("touch");
    std::env::set_var("HOME", home.path());
    let creds = Credentials {
        user: "alice".to_owned(),
        password: "secret".to_owned().into(),
        trusted_host_key: Some(fingerprint(b"host-key")),
    };
    let key = Auth::KeyFile(KeyFile::new("/tmp/id_ed25519").expect("key"));
    for auth in [Auth::Password, key, Auth::Agent] {
        let s = fixture_session(auth);
        assert!(connect_over(&FAKE_CLIENT_OPS, loopback_stream(), &s, &creds, "alice").is_ok());
    }
    restore_env("HOME", prev_home);
    restore_env("USERPROFILE", prev_profile);
}

#[test]
fn authenticate_and_verify_host_cover_unencrypted_keys_and_bad_known_hosts() {
    let _lock = crate::test_support::env_lock();
    let home = TempDir::new().expect("temp home");
    let prev_home = std::env::var_os("HOME");
    fs::create_dir_all(home.path().join(".ssh")).expect("mkdir .ssh");
    fs::write(home.path().join(".ssh/known_hosts"), b"").expect("touch");
    std::env::set_var("HOME", home.path());
    let key_s = fixture_session(Auth::KeyFile(KeyFile::new("/tmp/id_ed25519").expect("key")));
    let ok = (FAKE_CLIENT_OPS.new_session)().expect("fake");
    let empty = Credentials::default();
    assert!(authenticate(&FAKE_CLIENT_OPS, &ok, &key_s, &empty, "alice").is_ok());
    let bad = FakeSession {
        read_fails: true,
        check: CheckResult::NotFound,
        ..ok
    };
    let ag = fixture_session(Auth::Agent);
    assert_eq!(
        verify_host(&FAKE_CLIENT_OPS, &bad, &ag, &empty),
        Err(RemoteError::Failed("cannot read SSH known_hosts"))
    );
    restore_env("HOME", prev_home);
}

#[test]
fn real_ssh2_session_rejects_unconnected_password_agent_and_short_known_hosts() {
    let mut real = ssh2::Session::new().expect("ssh2 session");
    real.set_tcp_stream(loopback_stream());
    for auth in [Auth::Password, Auth::Agent] {
        let s = fixture_session(auth);
        assert!(authenticate(&SSH2_CLIENT_OPS, &real, &s, &Credentials::default(), "u").is_err());
    }
    let dir = TempDir::new().expect("tempdir");
    let kh_path = dir.path().join("known_hosts");
    fs::write(&kh_path, "[127.0.0.1]:2222 ssh-ed25519 short\n").expect("write");
    let mut kh = (SSH2_CLIENT_OPS.known_hosts)(&real).expect("known_hosts");
    assert!(
        (SSH2_CLIENT_OPS.read_known_hosts)(&mut kh, &kh_path, KnownHostFileKind::OpenSSH).is_err()
    );
}
