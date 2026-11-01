//! Purpose: Native SSH/SFTP authentication and host identity verification.

use super::{connection, Credentials, RemoteError, RemoteFs};
use crate::models::{Auth, RemotePath, Session};
use base64::Engine;
use sha2::{Digest, Sha256};
use ssh2::{CheckResult, KnownHostFileKind};
use std::path::{Path, PathBuf};

mod filesystem;
mod metadata;
#[cfg(test)]
mod tests;
#[cfg(test)]
#[cfg(target_os = "linux")]
mod tests_integration;

pub(super) struct SftpFs {
    sftp: ssh2::Sftp,
}

pub(super) fn connect(
    session: &Session,
    credentials: &Credentials,
    user: &str,
) -> Result<Box<dyn RemoteFs>, RemoteError> {
    let mut ssh = ssh2::Session::new()
        .map_err(|_protocol_error| RemoteError::Failed("cannot initialize SSH"))?;
    ssh.set_timeout(15_000);
    ssh.set_tcp_stream(connection::socket(session)?);
    ssh.handshake()
        .map_err(|_protocol_error| RemoteError::Failed("SSH handshake failed"))?;
    verify_host(&ssh, session, credentials)?;
    authenticate(&ssh, session, credentials, user)?;
    let sftp = ssh
        .sftp()
        .map_err(|_protocol_error| RemoteError::Failed("server refused the SFTP subsystem"))?;
    Ok(Box::new(SftpFs { sftp }))
}

fn authenticate(
    ssh: &ssh2::Session,
    session: &Session,
    credentials: &Credentials,
    user: &str,
) -> Result<(), RemoteError> {
    let result = match &session.auth {
        Auth::Password => ssh.userauth_password(user, &credentials.password),
        Auth::KeyFile(key) => ssh.userauth_pubkey_file(
            user,
            None,
            Path::new(&key.to_string()),
            (!credentials.password.is_empty()).then_some(credentials.password.as_str()),
        ),
        Auth::Agent => ssh.userauth_agent(user),
    };
    result.map_err(|_protocol_error| RemoteError::Failed("SSH authentication failed"))?;
    if !ssh.authenticated() {
        return Err(RemoteError::Failed("SSH authentication failed"));
    }
    Ok(())
}

fn verify_host(
    ssh: &ssh2::Session,
    session: &Session,
    credentials: &Credentials,
) -> Result<(), RemoteError> {
    let (key, _) = ssh
        .host_key()
        .ok_or(RemoteError::Failed("SSH server supplied no host key"))?;
    let fingerprint = fingerprint(key);
    let mut hosts = ssh
        .known_hosts()
        .map_err(|_protocol_error| RemoteError::Failed("cannot verify SSH host key"))?;
    if let Some(path) = known_hosts().filter(|path| path.exists()) {
        hosts
            .read_file(&path, KnownHostFileKind::OpenSSH)
            .map_err(|_protocol_error| RemoteError::Failed("cannot read SSH known_hosts"))?;
    }
    match hosts.check_port(&session.host.to_string(), session.port.get(), key) {
        CheckResult::Match => Ok(()),
        CheckResult::NotFound if credentials.trusted_host_key.as_ref() == Some(&fingerprint) => {
            Ok(())
        }
        CheckResult::NotFound => Err(RemoteError::HostKey(fingerprint)),
        CheckResult::Mismatch => Err(RemoteError::Failed(
            "SSH host key changed; connection refused",
        )),
        CheckResult::Failure => Err(RemoteError::Failed("cannot verify SSH host key")),
    }
}

fn known_hosts() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(|home| PathBuf::from(home).join(".ssh").join("known_hosts"))
}

fn fingerprint(key: &[u8]) -> String {
    let encoded = base64::engine::general_purpose::STANDARD_NO_PAD.encode(Sha256::digest(key));
    format!("SHA256:{encoded}")
}

fn at(error: &ssh2::Error, path: &RemotePath) -> RemoteError {
    match error.code() {
        ssh2::ErrorCode::SFTP(2) => RemoteError::NotFound(path.clone()),
        ssh2::ErrorCode::SFTP(3) => RemoteError::Failed("remote operation: permission denied"),
        ssh2::ErrorCode::SFTP(11) => RemoteError::AlreadyExists(path.clone()),
        _ => RemoteError::Failed("SFTP operation failed"),
    }
}
