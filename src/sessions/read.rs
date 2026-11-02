//! Purpose: Reading a small UTF-8 text file without opening what must not be
//! opened — the sessions file here, a `.netbox` export in the importer.

use std::fmt;
use std::io;
use std::path::Path;

use crate::fileload::{read_regular, Loaded};

/// Why a file this module reads was not read.
#[derive(Debug)]
pub enum ReadError {
    Io(io::Error),
    /// A directory, FIFO or device: never opened, since opening some of them
    /// blocks forever.
    NotAFile,
    TooLarge {
        limit: u64,
    },
    NotUtf8,
}

impl fmt::Display for ReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "{error}"),
            Self::NotAFile => f.write_str("not a regular file"),
            Self::TooLarge { limit } => write!(f, "larger than {limit} bytes"),
            Self::NotUtf8 => f.write_str("not UTF-8 text"),
        }
    }
}

/// The text of the regular file at `path`, if it is at most `limit` bytes of
/// UTF-8.
#[expect(
    clippy::map_err_ignore,
    reason = "the offset of the bad UTF-8 sequence means nothing to the user; the fixed message does"
)]
pub fn read_text(path: &Path, limit: u64) -> Result<String, ReadError> {
    match read_regular(path, limit).map_err(ReadError::Io)? {
        Loaded::Bytes(bytes) => String::from_utf8(bytes).map_err(|_| ReadError::NotUtf8),
        Loaded::NotRegular => Err(ReadError::NotAFile),
        Loaded::TooLarge(_) => Err(ReadError::TooLarge { limit }),
    }
}
