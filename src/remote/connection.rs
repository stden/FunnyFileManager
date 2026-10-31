//! Purpose: Connect stored sessions with transient credentials and bounded socket I/O.

use super::{RemoteError, RemoteFs};
use crate::models::{Protocol, Session};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

pub(super) const TIMEOUT: Duration = Duration::from_secs(15);

/// Transient connection input. It is never serialized or formatted for logs.
#[derive(Clone, Default)]
pub struct Credentials {
    pub user: String,
    pub password: zeroize::Zeroizing<String>,
    /// An unknown host is accepted only after this exact fingerprint was confirmed.
    pub trusted_host_key: Option<String>,
}

pub fn connect_with(
    session: &Session,
    credentials: &Credentials,
) -> Result<Box<dyn RemoteFs>, RemoteError> {
    if matches!(session.protocol, Protocol::Scp | Protocol::Ftps) {
        return Err(RemoteError::NoTransport(session.protocol));
    }
    let user = if credentials.user.is_empty() {
        session
            .user
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_default()
    } else {
        credentials.user.clone()
    };
    if user.is_empty() {
        return Err(RemoteError::AuthenticationRequired);
    }
    match session.protocol {
        Protocol::Sftp => super::sftp::connect(session, credentials, &user),
        Protocol::Ftp => super::ftp::connect(session, credentials, &user),
        Protocol::Scp | Protocol::Ftps => Err(RemoteError::NoTransport(session.protocol)),
    }
}

pub(super) fn socket(session: &Session) -> Result<TcpStream, RemoteError> {
    let host = session.host.to_string();
    let addresses = (host.as_str(), session.port.get())
        .to_socket_addrs()
        .map_err(|_protocol_error| RemoteError::Failed("cannot resolve the remote host"))?;
    for address in addresses {
        if let Ok(socket) = TcpStream::connect_timeout(&address, TIMEOUT) {
            socket.set_read_timeout(Some(TIMEOUT))?;
            socket.set_write_timeout(Some(TIMEOUT))?;
            return Ok(socket);
        }
    }
    Err(RemoteError::Failed("cannot connect to the remote host"))
}
