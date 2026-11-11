//! Purpose: The paths a registration writes into a config — the server binary
//! and the folder it is confined to — as newtypes that can only hold what a
//! JSON or TOML file can carry.

use std::path::{Path, PathBuf};

/// The absolute path of an `ffm-mcp` that exists, as valid Unicode.
///
/// Kept as a `String`: JSON and TOML hold text, and a path that is not valid
/// Unicode cannot be written into either, so the check happens once, here,
/// rather than as a lossy conversion at every write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerBinary(String);

impl ServerBinary {
    /// The server at `path`, or the path back when it is not valid Unicode.
    pub fn new(path: PathBuf) -> Result<Self, PathBuf> {
        path.into_os_string()
            .into_string()
            .map(Self)
            .map_err(PathBuf::from)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[must_use]
    pub fn as_path(&self) -> &Path {
        Path::new(&self.0)
    }
}

/// The absolute path of a local folder, as valid Unicode, in its ordinary
/// spelling: never Windows' `\\?\` form, which a canonicalized panel path
/// has and nothing a client writes does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectDir(String);

impl ProjectDir {
    /// `None` unless `path` is absolute and valid Unicode. Whether the folder
    /// exists is the caller's question: this module does not look.
    #[must_use]
    pub fn new(path: PathBuf) -> Option<Self> {
        if !path.is_absolute() {
            return None;
        }
        let text = path.into_os_string().into_string().ok()?;
        Some(Self(
            crate::path_display::without_verbatim_prefix(&text).into_owned(),
        ))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[must_use]
    pub fn as_path(&self) -> &Path {
        Path::new(&self.0)
    }
}

/// What a registration writes: `root` is `Some` exactly at
/// [`Scope::CurrentFolder`](super::Scope::CurrentFolder).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerEntry {
    pub command: ServerBinary,
    pub root: Option<ProjectDir>,
}

/// Where the server was looked for, and what was there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServerState {
    Found(ServerBinary),
    /// Neither next to `ffm` nor on PATH.
    Missing,
    /// Found, but its path cannot be written into a config.
    NotUnicode(PathBuf),
}

impl ServerState {
    /// The server, when there is one a config can name.
    #[must_use]
    pub const fn binary(&self) -> Option<&ServerBinary> {
        match self {
            Self::Found(binary) => Some(binary),
            Self::Missing | Self::NotUnicode(_) => None,
        }
    }
}
