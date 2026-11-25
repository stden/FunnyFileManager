//! Purpose: Whether the Language servers dialog may write the settings file,
//! and why it only shows the servers when it may not.

use std::path::PathBuf;

/// Whether the dialog may write the settings file, and which one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Access {
    Writable { path: PathBuf },
    ReadOnly(ReadOnlyReason),
}

/// Why the dialog only shows the servers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadOnlyReason {
    /// A session without a home keeps its settings in memory.
    NoSettingsFile,
    /// The file at `path` cannot be read, or does not parse; `reason` says
    /// why, naming the file.
    Document { path: PathBuf, reason: String },
}

impl std::fmt::Display for ReadOnlyReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoSettingsFile => f.write_str("no settings file for this session"),
            Self::Document { reason, .. } => f.write_str(reason),
        }
    }
}
