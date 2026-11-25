//! Purpose: Portable remote facts and validated Linux permission bits.

use super::RemoteError;
use crate::values::entry::{DirEntry, EntryKind, FileSize, Modified};

/// Linux permission bits, including setuid, setgid and sticky.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnixMode(u32);

impl UnixMode {
    pub fn parse(text: &str) -> Result<Self, RemoteError> {
        if text.is_empty()
            || text.len() > 4
            || !text.bytes().all(|byte| matches!(byte, b'0'..=b'7'))
        {
            return Err(RemoteError::InvalidInput(
                "permissions must be 1–4 octal digits (0000–7777)",
            ));
        }
        let bits = text
            .bytes()
            .fold(0_u32, |acc, byte| (acc << 3) | u32::from(byte - b'0'));
        Ok(Self(bits))
    }

    pub const fn from_mode(bits: u32) -> Self {
        Self(bits & 0o7777)
    }
    pub const fn bits(self) -> u32 {
        self.0
    }
}

impl std::fmt::Display for UnixMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:04o}", self.0)
    }
}

/// Metadata uses Linux modes even when the client runs on Windows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteMetadata {
    pub kind: EntryKind,
    pub size: FileSize,
    pub modified: Modified,
    pub mode: Option<UnixMode>,
}

impl RemoteMetadata {
    pub fn from_entry(entry: &DirEntry) -> Self {
        Self {
            kind: entry.kind,
            size: entry.size,
            modified: entry.modified,
            mode: (entry.attributes.bits() != 0)
                .then(|| UnixMode::from_mode(entry.attributes.bits())),
        }
    }
}

/// A taken name is overwritten only after an explicit overwrite choice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteMode {
    CreateNew,
    Overwrite,
}
