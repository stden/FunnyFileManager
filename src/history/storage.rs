//! Purpose: Where the history file lives — the persistence target and its XDG path.

use std::path::PathBuf;

/// Where this history writes, if anywhere.
///
/// An enum rather than `Option<PathBuf>` because `None` here does not mean
/// "the path is missing" — it means *this history deliberately does not
/// persist*, which is the state every test and every freshly-constructed `App`
/// is in. `save` is then not a failed write, it is a no-op by design, and the
/// name says so.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Persistence {
    /// Nothing reaches disk; the ring dies with the process.
    #[default]
    SessionOnly,
    /// Durable lines are written to this file.
    File(PathBuf),
}

impl Persistence {
    /// The file to write to, or `None` while session-only.
    #[must_use]
    pub fn file(&self) -> Option<&std::path::Path> {
        match self {
            Self::SessionOnly => None,
            Self::File(path) => Some(path),
        }
    }
}

/// `$XDG_DATA_HOME/ffm/history`, defaulting to `~/.local/share/ffm/history`.
pub(super) fn history_file_path() -> Option<PathBuf> {
    data_file("history")
}

/// `$XDG_DATA_HOME/ffm/<name>`, defaulting to `~/.local/share/ffm/<name>` —
/// the one data directory every history (command, view, folder) shares.
/// `None` without an absolute home (see [`crate::config::data_dir`]), so a
/// history never lands in a cwd-relative directory.
pub fn data_file(name: &str) -> Option<PathBuf> {
    crate::config::data_dir().map(|dir| dir.join(name))
}
