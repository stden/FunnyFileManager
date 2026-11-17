//! Purpose: Binary passive FTP transfers and Linux SITE CHMOD, without credential logs.

use super::{connection, Credentials, RemoteError, RemoteFs, UnixMode, WriteMode};
use crate::values::{entry::DirEntry, remote_path::RemotePath, session::Session};
use std::{
    io::{BufReader, BufWriter, Read, Write},
    net::{SocketAddr, TcpStream},
};
use suppaftp::{FtpError, FtpStream, TlsStream, TransferStream};

const FTP_IO_BUFFER_BYTES: usize = 256 * 1024;
/// RFC 959 reply codes indicating the server does not implement an optional command.
const RFC_SUPERFLUOUS_COMMAND: u32 = 202;
const RFC_COMMAND_UNRECOGNIZED: u32 = 500;
const RFC_PARAMETER_SYNTAX_ERROR: u32 = 501;
const RFC_COMMAND_NOT_IMPLEMENTED: u32 = 502;
const RFC_PARAMETER_NOT_IMPLEMENTED: u32 = 504;

mod listing;
#[cfg(test)]
mod tests;

struct FtpFs {
    client: FtpStream,
}

pub(super) fn connect(
    session: &Session,
    credentials: &Credentials,
    user: &str,
) -> Result<Box<dyn RemoteFs>, RemoteError> {
    let socket = connection::socket(session)?;
    let peer = socket.peer_addr()?.ip();
    let mut client = FtpStream::connect_with_stream(socket)
        .map_err(failed)?
        .passive_stream_builder(move |address| {
            let addr = SocketAddr::new(peer, address.port());
            let socket = TcpStream::connect_timeout(&addr, connection::TIMEOUT)
                .map_err(FtpError::ConnectionError)?;
            socket
                .set_read_timeout(Some(connection::TIMEOUT))
                .map_err(FtpError::ConnectionError)?;
            socket
                .set_write_timeout(Some(connection::TIMEOUT))
                .map_err(FtpError::ConnectionError)?;
            Ok(socket)
        });
    client
        .login(user, credentials.password.as_str())
        .map_err(|_protocol_error| RemoteError::Failed("FTP authentication failed"))?;
    client
        .transfer_type(suppaftp::types::FileType::Binary)
        .map_err(failed)?;
    Ok(Box::new(FtpFs { client }))
}

impl RemoteFs for FtpFs {
    fn list(&mut self, dir: &RemotePath) -> Result<Vec<DirEntry>, RemoteError> {
        let dir = argument(dir)?;
        match self.client.mlsd(Some(dir)) {
            Ok(lines) => listing::parse(&lines, true),
            Err(error) if unsupported(&error) => {
                let lines = self.client.list(Some(dir)).map_err(failed)?;
                listing::parse(&lines, false)
            }
            Err(error) => Err(failed(error)),
        }
    }

    fn download(&mut self, path: &RemotePath, output: &mut dyn Write) -> Result<u64, RemoteError> {
        let mut stream = self
            .client
            .retr_as_stream(argument(path)?)
            .map_err(failed)?;
        let copied = std::io::copy(
            &mut BufReader::with_capacity(FTP_IO_BUFFER_BYTES, &mut stream),
            output,
        );
        finish_transfer(stream, copied)
    }

    fn upload(
        &mut self,
        path: &RemotePath,
        input: &mut dyn Read,
        mode: WriteMode,
    ) -> Result<u64, RemoteError> {
        let remote_path = argument(path)?;
        if mode == WriteMode::CreateNew {
            self.require_missing(path)?;
        }
        let mut stream = self.client.put_with_stream(remote_path).map_err(failed)?;
        let copied = copy_buffered_write(input, &mut stream);
        finish_transfer(stream, copied)
    }

    fn mkdir(&mut self, path: &RemotePath) -> Result<(), RemoteError> {
        self.client.mkdir(argument(path)?).map_err(failed)
    }

    fn remove_file(&mut self, path: &RemotePath) -> Result<(), RemoteError> {
        self.client.rm(argument(path)?).map_err(failed)
    }

    fn rmdir(&mut self, path: &RemotePath) -> Result<(), RemoteError> {
        self.client.rmdir(argument(path)?).map_err(failed)
    }

    fn rename(&mut self, from: &RemotePath, to: &RemotePath) -> Result<(), RemoteError> {
        let from_arg = argument(from)?;
        let to_arg = argument(to)?;
        self.require_missing(to)?;
        self.client.rename(from_arg, to_arg).map_err(failed)
    }

    fn chmod(&mut self, path: &RemotePath, mode: UnixMode) -> Result<(), RemoteError> {
        match self
            .client
            .site(format!("CHMOD {mode} {}", argument(path)?))
        {
            Ok(_) => Ok(()),
            Err(error) if unsupported(&error) => Err(RemoteError::Unsupported("SITE CHMOD")),
            Err(error) => Err(failed(error)),
        }
    }

    fn replace_file(
        &mut self,
        temporary: &RemotePath,
        target: &RemotePath,
    ) -> Result<(), RemoteError> {
        self.client
            .rename(argument(temporary)?, argument(target)?)
            .map_err(failed)
    }
}

impl FtpFs {
    fn require_missing(&mut self, path: &RemotePath) -> Result<(), RemoteError> {
        match self.stat(path) {
            Err(RemoteError::NotFound(_)) => Ok(()),
            Ok(_) => Err(RemoteError::AlreadyExists(path.clone())),
            Err(error) => Err(error),
        }
    }
}

fn copy_buffered_write(input: &mut dyn Read, output: &mut impl Write) -> std::io::Result<u64> {
    let mut buffered = BufWriter::with_capacity(FTP_IO_BUFFER_BYTES, output);
    let bytes = std::io::copy(input, &mut buffered)?;
    buffered.flush()?;
    Ok(bytes)
}

/// Close the passive data socket and read the server's completion reply before
/// propagating `copied?`, keeping the FTP control channel synchronized on I/O errors.
fn finish_transfer<T: TlsStream>(
    stream: TransferStream<T>,
    copied: std::io::Result<u64>,
) -> Result<u64, RemoteError> {
    let finished = stream.finish().map_err(failed);
    let bytes = copied?;
    finished?;
    Ok(bytes)
}

fn argument(path: &RemotePath) -> Result<&str, RemoteError> {
    if path.as_str().chars().any(char::is_control) {
        return Err(RemoteError::InvalidInput(
            "FTP cannot address names containing control characters",
        ));
    }
    Ok(path.as_str())
}

fn unsupported(error: &FtpError) -> bool {
    matches!(
        error,
        FtpError::UnexpectedResponse(reply)
            if matches!(
                reply.status.code(),
                RFC_SUPERFLUOUS_COMMAND
                    | RFC_COMMAND_UNRECOGNIZED
                    | RFC_PARAMETER_SYNTAX_ERROR
                    | RFC_COMMAND_NOT_IMPLEMENTED
                    | RFC_PARAMETER_NOT_IMPLEMENTED
            )
    )
}

fn failed(error: FtpError) -> RemoteError {
    match error {
        FtpError::ConnectionError(error) => error.into(),
        _ => RemoteError::Failed("FTP server refused the operation"),
    }
}
