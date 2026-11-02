//! Purpose: Preserve application refusals while recognizing broken child transports.

use std::fmt;

#[derive(Debug)]
pub enum Error {
    Invalid(&'static str),
    Io(std::io::Error),
    Protocol(&'static str),
    Application { code: String, message: String },
    UnknownSession,
    SessionLimit,
    Timeout,
}

impl Error {
    pub(super) const fn is_fatal(&self) -> bool {
        matches!(self, Self::Io(_) | Self::Protocol(_) | Self::Timeout)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(message) | Self::Protocol(message) => formatter.write_str(message),
            Self::Io(error) => write!(formatter, "FFM automation transport: {error}"),
            Self::Application { code, message } => write!(formatter, "{code}: {message}"),
            Self::UnknownSession => formatter.write_str("unknown or closed FFM session"),
            Self::SessionLimit => formatter.write_str("maximum of four FFM sessions reached"),
            Self::Timeout => {
                formatter.write_str("FFM automation response timed out; session closed")
            }
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}
