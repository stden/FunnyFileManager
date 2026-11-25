//! Purpose: Native explicit and implicit FTPS over rustls with system-independent root CAs.

use super::connection;
use crate::remote::{Credentials, RemoteError, RemoteFs};
use crate::values::session::Session;
use std::net::IpAddr;
use std::sync::Arc;
use suppaftp::rustls::crypto::aws_lc_rs;
use suppaftp::rustls::{ClientConfig, RootCertStore};
use suppaftp::{FtpError, RustlsConnector, RustlsFtpStream};

pub(super) fn default_tls_config() -> Result<Arc<ClientConfig>, RemoteError> {
    build_tls_config(suppaftp::rustls::DEFAULT_VERSIONS)
}

pub(super) fn build_tls_config(
    versions: &[&'static suppaftp::rustls::SupportedProtocolVersion],
) -> Result<Arc<ClientConfig>, RemoteError> {
    let mut roots = RootCertStore::empty();
    roots.add_parsable_certificates(webpki_root_certs::TLS_SERVER_ROOT_CERTS.iter().cloned());
    let provider = Arc::new(aws_lc_rs::default_provider());
    let config = ClientConfig::builder_with_provider(provider)
        .with_protocol_versions(versions)
        .map_err(|_config_error| RemoteError::Failed("FTPS TLS configuration failed"))?
        .with_root_certificates(roots)
        .with_no_client_auth();
    Ok(Arc::new(config))
}

/// Connect to an FTPS server using implicit TLS on port 990 or explicit `AUTH TLS` otherwise.
pub fn connect(
    session: &Session,
    credentials: &Credentials,
    user: &str,
) -> Result<Box<dyn RemoteFs>, RemoteError> {
    connect_with_config(
        session,
        credentials,
        user,
        session.port.get() == 990,
        default_tls_config()?,
    )
}

pub(super) fn connect_with_config(
    session: &Session,
    credentials: &Credentials,
    user: &str,
    implicit: bool,
    config: Arc<ClientConfig>,
) -> Result<Box<dyn RemoteFs>, RemoteError> {
    let connector = RustlsConnector::from(config);
    let domain = session.host.as_str();
    let (client, peer) = if implicit {
        let client = RustlsFtpStream::connect_secure_implicit(
            (domain, session.port.get()),
            connector,
            domain,
        )
        .map_err(tls_failed)?;
        let peer = configure_socket(&client)?;
        (client, peer)
    } else {
        let socket = connection::socket(session)?;
        let peer = socket.peer_addr()?.ip();
        let client = RustlsFtpStream::connect_with_stream(socket).map_err(super::failed)?;
        let client = client.into_secure(connector, domain).map_err(tls_failed)?;
        (client, peer)
    };
    super::finish_login(client, peer, user, credentials.password.as_str())
}

fn configure_socket(client: &RustlsFtpStream) -> Result<IpAddr, RemoteError> {
    let socket = client.get_ref();
    socket.set_read_timeout(Some(connection::TIMEOUT))?;
    socket.set_write_timeout(Some(connection::TIMEOUT))?;
    Ok(socket.peer_addr()?.ip())
}

pub(super) fn tls_failed(error: FtpError) -> RemoteError {
    match error {
        FtpError::ConnectionError(ref io_err) if is_tls_error(io_err) => {
            RemoteError::Failed("FTPS TLS handshake failed")
        }
        FtpError::ConnectionError(error) => error.into(),
        FtpError::SecureError(_) => RemoteError::Failed("FTPS TLS handshake failed"),
        _ => RemoteError::Failed("FTP server refused TLS upgrade"),
    }
}

fn is_tls_error(error: &std::io::Error) -> bool {
    error
        .get_ref()
        .is_some_and(<dyn std::error::Error + Send + Sync + 'static>::is::<suppaftp::rustls::Error>)
}
