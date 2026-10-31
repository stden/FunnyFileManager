//! Purpose: Binary passive FTP transfers and Linux SITE CHMOD, without credential logs.

use super::{connection, Credentials, RemoteError, RemoteFs, UnixMode, WriteMode};
use crate::models::{DirEntry, RemotePath, Session};
use std::{
    io::{Read, Write},
    net::TcpStream,
};
use suppaftp::{FtpError, FtpStream};

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
            let socket = TcpStream::connect_timeout(
                &std::net::SocketAddr::new(peer, address.port()),
                connection::TIMEOUT,
            )
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
        let copied = std::io::copy(&mut stream, output);
        let finished = stream.finish().map_err(failed);
        let bytes = copied?;
        finished?;
        Ok(bytes)
    }

    fn upload(
        &mut self,
        path: &RemotePath,
        input: &mut dyn Read,
        mode: WriteMode,
    ) -> Result<u64, RemoteError> {
        argument(path)?;
        if mode == WriteMode::CreateNew {
            self.require_missing(path)?;
        }
        let mut stream = self.client.put_with_stream(path.as_str()).map_err(failed)?;
        let copied = std::io::copy(input, &mut stream);
        let finished = stream.finish().map_err(failed);
        let bytes = copied?;
        finished?;
        Ok(bytes)
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
        argument(from)?;
        argument(to)?;
        self.require_missing(to)?;
        self.client
            .rename(from.as_str(), to.as_str())
            .map_err(failed)
    }

    fn chmod(&mut self, path: &RemotePath, mode: UnixMode) -> Result<(), RemoteError> {
        let response = self
            .client
            .site(format!("CHMOD {mode} {}", argument(path)?))
            .map_err(failed)?;
        if response.status.code() / 100 != 2 {
            return Err(RemoteError::Unsupported("SITE CHMOD"));
        }
        Ok(())
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

fn argument(path: &RemotePath) -> Result<&str, RemoteError> {
    if path.as_str().chars().any(char::is_control) {
        return Err(RemoteError::InvalidInput(
            "FTP cannot address names containing control characters",
        ));
    }
    Ok(path.as_str())
}

fn unsupported(error: &FtpError) -> bool {
    matches!(error, FtpError::UnexpectedResponse(reply) if matches!(reply.status.code(), 500 | 501 | 502 | 504))
}

fn failed(error: FtpError) -> RemoteError {
    match error {
        FtpError::ConnectionError(error) => error.into(),
        _ => RemoteError::Failed("FTP server refused the operation"),
    }
}
