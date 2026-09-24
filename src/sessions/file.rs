//! Purpose: Loading the sessions file and writing it back — never over a file
//! that did not load.

use std::collections::BTreeSet;
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

use super::{read_text, Backing, ReadError, SessionStore, StoredFile, FILE_NAME, MAX_FILE_BYTES};
use crate::models::{Session, SessionName};

/// Why the store was not written.
#[derive(Debug)]
pub enum StoreError {
    /// The file did not load at start-up, so it is kept as it is.
    Damaged,
    /// The serialized store would be too large to load again.
    TooLarge {
        limit: u64,
    },
    Write(io::Error),
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Damaged => f.write_str(
                "the sessions file did not load, so it is not overwritten; fix or move it first",
            ),
            Self::TooLarge { limit } => {
                write!(f, "the sessions file would exceed {limit} bytes")
            }
            Self::Write(error) => write!(f, "cannot write the sessions file: {error}"),
        }
    }
}

impl SessionStore {
    /// `<config dir>/ffm/sessions.json`; `None` when the environment names no
    /// home at all.
    #[must_use]
    pub fn path() -> Option<PathBuf> {
        crate::config::dir().map(|dir| dir.join(FILE_NAME))
    }

    /// The saved sessions, from [`Self::path`].
    #[must_use]
    pub fn load() -> Self {
        Self::path().map_or_else(Self::default, Self::load_from)
    }

    /// The sessions in the file at `path`: none when there is no file, and
    /// none — in a store that will not save — when the file is unusable. Why
    /// it was unusable goes to the log, never to the screen.
    #[must_use]
    pub fn load_from(path: PathBuf) -> Self {
        match read_sessions(&path) {
            Ok(sessions) => Self {
                sessions,
                backing: Backing::File(path),
            },
            Err(reason) => {
                crate::diagnostics::log_error(&format!(
                    "sessions: {} did not load: {reason}",
                    path.display()
                ));
                Self {
                    sessions: Vec::new(),
                    backing: Backing::Damaged(path),
                }
            }
        }
    }

    /// Whether the file existed and did not load.
    #[must_use]
    pub const fn is_damaged(&self) -> bool {
        matches!(self.backing, Backing::Damaged(_))
    }

    /// Write the store where it was loaded from: a sibling temp file with
    /// mode `0600` replaces the old one in a single rename. A session-only
    /// store has nowhere to go and succeeds; a damaged one refuses.
    pub fn save(&self) -> Result<(), StoreError> {
        let path = match &self.backing {
            Backing::SessionOnly => return Ok(()),
            Backing::Damaged(_) => return Err(StoreError::Damaged),
            Backing::File(path) => path,
        };
        let text = serde_json::to_string_pretty(&StoredFile {
            sessions: self.sessions.clone(),
        })
        .map_err(|error| StoreError::Write(io::Error::other(error)))?;
        if u64::try_from(text.len()).unwrap_or(u64::MAX) > MAX_FILE_BYTES {
            return Err(StoreError::TooLarge {
                limit: MAX_FILE_BYTES,
            });
        }
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(StoreError::Write)?;
        }
        crate::history::write_owner_only(path, &text).map_err(StoreError::Write)
    }
}

/// Why a sessions file did not load; for the log.
#[derive(Debug)]
pub(super) enum LoadError {
    Read(ReadError),
    Parse(serde_json::Error),
    DuplicateName,
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read(error) => write!(f, "{error}"),
            Self::Parse(error) => write!(f, "{error}"),
            Self::DuplicateName => f.write_str("two sessions share a name"),
        }
    }
}

/// The sessions in `path`, or none when there is no file.
pub(super) fn read_sessions(path: &Path) -> Result<Vec<Session>, LoadError> {
    let text = match read_text(path, MAX_FILE_BYTES) {
        Ok(text) => text,
        Err(ReadError::Io(error)) if error.kind() == io::ErrorKind::NotFound => {
            return Ok(Vec::new())
        }
        Err(error) => return Err(LoadError::Read(error)),
    };
    let file: StoredFile = serde_json::from_str(&text).map_err(LoadError::Parse)?;
    let names: BTreeSet<&SessionName> = file.sessions.iter().map(|s| &s.name).collect();
    if names.len() != file.sessions.len() {
        return Err(LoadError::DuplicateName);
    }
    Ok(file.sessions)
}
