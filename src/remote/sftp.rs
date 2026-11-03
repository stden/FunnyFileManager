//! Purpose: Native SSH/SFTP authentication and host identity verification.

use super::{connection, Credentials, RemoteError, RemoteFs};
use crate::values::remote_path::RemotePath;
use crate::values::session::{Auth, Session};
use base64::Engine;
use ops::{ClientOps, SftpOps, SSH2_CLIENT_OPS};
use sha2::{Digest, Sha256};
use ssh2::{CheckResult, KnownHostFileKind};
use std::{
    io::{Read, Write},
    net::TcpStream,
    path::{Path, PathBuf},
};

mod filesystem;
mod metadata;
mod ops;
#[cfg(test)]
mod tests;
#[cfg(test)]
#[cfg(target_os = "linux")]
mod tests_integration;

pub(super) struct SftpFs<S: 'static, F: 'static> {
    sftp: S,
    ops: &'static SftpOps<S, F>,
}

pub(super) fn connect(
    session: &Session,
    credentials: &Credentials,
    user: &str,
) -> Result<Box<dyn RemoteFs>, RemoteError> {
    connect_over(
        &SSH2_CLIENT_OPS,
        connection::socket(session)?,
        session,
        credentials,
        user,
    )
}

fn connect_over<S: 'static, K, B: Send + 'static, F: Read + Write + 'static>(
    ops: &ClientOps<S, K, B, F>,
    stream: TcpStream,
    session: &Session,
    credentials: &Credentials,
    user: &str,
) -> Result<Box<dyn RemoteFs>, RemoteError> {
    let mut ssh = ssh_result((ops.new_session)(), "cannot initialize SSH")?;
    (ops.set_timeout)(&ssh, 15_000);
    (ops.set_tcp_stream)(&mut ssh, stream);
    ssh_result((ops.handshake)(&mut ssh), "SSH handshake failed")?;
    verify_host(ops, &ssh, session, credentials)?;
    authenticate(ops, &ssh, session, credentials, user)?;
    let sftp = ssh_result((ops.open_sftp)(&ssh), "server refused the SFTP subsystem")?;
    Ok(Box::new(SftpFs {
        sftp,
        ops: ops.sftp_ops,
    }))
}

fn authenticate<S: 'static, K, B: 'static, F: 'static>(
    ops: &ClientOps<S, K, B, F>,
    ssh: &S,
    session: &Session,
    credentials: &Credentials,
    user: &str,
) -> Result<(), RemoteError> {
    let result = match &session.auth {
        Auth::Password => (ops.auth_password)(ssh, user, &credentials.password),
        Auth::KeyFile(key) => (ops.auth_pubkey_file)(
            ssh,
            user,
            None,
            Path::new(&key.to_string()),
            (!credentials.password.is_empty()).then_some(credentials.password.as_str()),
        ),
        Auth::Agent => (ops.auth_agent)(ssh, user),
    };
    authentication_result(result, (ops.authenticated)(ssh))
}

fn authentication_result(
    result: Result<(), ssh2::Error>,
    authenticated: bool,
) -> Result<(), RemoteError> {
    ssh_result(result, "SSH authentication failed")?;
    if !authenticated {
        return Err(RemoteError::Failed("SSH authentication failed"));
    }
    Ok(())
}

fn verify_host<S: 'static, K, B: 'static, F: 'static>(
    ops: &ClientOps<S, K, B, F>,
    ssh: &S,
    session: &Session,
    credentials: &Credentials,
) -> Result<(), RemoteError> {
    let (key, _) =
        (ops.host_key)(ssh).ok_or(RemoteError::Failed("SSH server supplied no host key"))?;
    let fingerprint = fingerprint(key);
    let mut hosts = ssh_result((ops.known_hosts)(ssh), "cannot verify SSH host key")?;
    if let Some(path) = known_hosts().filter(|path| path.exists()) {
        ssh_result(
            (ops.read_known_hosts)(&mut hosts, &path, KnownHostFileKind::OpenSSH),
            "cannot read SSH known_hosts",
        )?;
    }
    host_key_decision(
        (ops.check_port)(&hosts, &session.host.to_string(), session.port.get(), key),
        fingerprint,
        credentials.trusted_host_key.as_deref(),
    )
}

fn host_key_decision(
    result: CheckResult,
    fingerprint: String,
    trusted: Option<&str>,
) -> Result<(), RemoteError> {
    match result {
        CheckResult::Match => Ok(()),
        CheckResult::NotFound if trusted == Some(fingerprint.as_str()) => Ok(()),
        CheckResult::NotFound => Err(RemoteError::HostKey(fingerprint)),
        CheckResult::Mismatch => Err(RemoteError::Failed(
            "SSH host key changed; connection refused",
        )),
        CheckResult::Failure => Err(RemoteError::Failed("cannot verify SSH host key")),
    }
}

fn ssh_result<T>(result: Result<T, ssh2::Error>, message: &'static str) -> Result<T, RemoteError> {
    result.map_err(|_protocol_error| RemoteError::Failed(message))
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
