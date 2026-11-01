//! Purpose: The file a copy or a move is sending, as the progress dialog shows
//! it: where it goes, and how much of it is copied.

use std::path::PathBuf;

use super::permille_of;

/// How far the file being sent has got.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum FileProgress {
    /// Being copied: `done` of its `size` bytes. A rename copies nothing, so it
    /// stays here until it is done.
    Copying { done: u64, size: u64 },
    /// Done, whatever its size. A file of no bytes has nothing to count, so
    /// this is the only way for it to fill its bar.
    Done,
}

impl FileProgress {
    /// Thousandths of the file copied, never above 1000.
    #[must_use]
    pub fn permille(self) -> u16 {
        match self {
            Self::Copying { done, size } => permille_of(done, size),
            Self::Done => 1000,
        }
    }

    /// The file with `bytes` more of it copied.
    #[must_use]
    pub const fn advanced(self, bytes: u64) -> Self {
        match self {
            Self::Copying { done, size } => Self::Copying {
                done: done.saturating_add(bytes),
                size,
            },
            Self::Done => Self::Done,
        }
    }
}

/// The file a copy or a move is sending now, and where it is going.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Transfer {
    pub destination: PathBuf,
    pub file: FileProgress,
}
