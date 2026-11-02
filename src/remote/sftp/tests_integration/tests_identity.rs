use super::{connect_with, fixture::Server, Credentials, RemoteError};
use crate::models::remote_work::{RemoteOutcome, RemoteTask};
use crate::models::{Auth, Session};
use std::path::Path;
use std::{env, fs};

pub(super) fn check(session: &Session, credentials: &Credentials, server: &Server) {
    let mut selected = session.clone();
    selected.remote_dir = Some(server.path(""));
    let task = RemoteTask::Connect {
        session: selected.clone(),
        credentials: credentials.clone(),
    };
    assert!(
        matches!(crate::ops::remote::jobs::execute(&task), Ok(RemoteOutcome::Listed { path, .. }) if path == server.path(""))
    );
    selected.remote_dir = Some(server.path("missing-start-directory"));
    assert!(crate::ops::remote::jobs::execute(&RemoteTask::Connect {
        session: selected,
        credentials: credentials.clone()
    })
    .is_err());
    for auth in [Auth::Password, Auth::Agent] {
        let mut rejected = session.clone();
        rejected.auth = auth;
        assert!(matches!(
            connect_with(&rejected, credentials),
            Err(RemoteError::Failed("SSH authentication failed"))
        ));
    }
    known_host_entries(session, credentials, server);
}

fn known_host_entries(session: &Session, credentials: &Credentials, server: &Server) {
    assert_eq!(env::var(super::CHILD).as_deref(), Ok("1"));
    let known_hosts = std::path::PathBuf::from(env::var_os("HOME").expect("isolated home"))
        .join(".ssh/known_hosts");
    fs::create_dir_all(known_hosts.parent().expect("SSH directory")).expect("SSH directory");
    let key = fs::read_to_string(server.local("host.pub")).expect("public host key");
    fs::write(
        &known_hosts,
        format!("[{}]:{} {key}", session.host, session.port.get()),
    )
    .expect("trusted host entry");
    let unconfirmed = Credentials {
        trusted_host_key: None,
        ..credentials.clone()
    };
    assert!(
        connect_with(session, &unconfirmed).is_ok(),
        "known identity needs no new confirmation"
    );
    reject_changed_key(session, credentials, server, &known_hosts);
    reject_malformed_record(session, credentials, &known_hosts);
}

fn reject_changed_key(
    session: &Session,
    credentials: &Credentials,
    server: &Server,
    known_hosts: &Path,
) {
    let wrong = fs::read_to_string(server.local("client.pub")).expect("different public key");
    fs::write(
        known_hosts,
        format!("[{}]:{} {wrong}", session.host, session.port.get()),
    )
    .expect("mismatched entry");
    assert!(matches!(
        connect_with(session, credentials),
        Err(RemoteError::Failed(
            "SSH host key changed; connection refused"
        ))
    ));
}

fn reject_malformed_record(session: &Session, credentials: &Credentials, known_hosts: &Path) {
    fs::write(
        known_hosts,
        format!(
            "[{}]:{} ssh-ed25519 invalid-key-data!\n",
            session.host,
            session.port.get()
        ),
    )
    .expect("malformed entry");
    assert!(matches!(
        connect_with(session, credentials),
        Err(RemoteError::Failed("cannot read SSH known_hosts"))
    ));
}
