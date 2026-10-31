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
    /// The file did not load at start-up, for this reason, so it is kept as
    /// it is.
    Damaged(String),
    /// The serialized store would be too large to load again.
    TooLarge {
        limit: u64,
    },
    Write(io::Error),
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Damaged(reason) => write!(
                f,
                "{FILE_NAME} did not load ({reason}), so it is not overwritten; fix or move it first"
            ),
            Self::TooLarge { limit } => {
                write!(f, "the sessions file would exceed {limit} bytes")
            }
            Self::Write(error) => write!(f, "cannot write the sessions file: {error}"),
        }
    }
}

impl SessionStore {
    /// `<config dir>/ffm/ffm.yml`; `None` when the environment names no
    /// home at all.
    #[must_use]
    pub fn path() -> Option<PathBuf> {
        crate::config::dir().map(|dir| dir.join(super::yaml::USER_FILE_NAME))
    }

    /// The saved sessions, from [`Self::path`].
    #[must_use]
    pub fn load() -> Self {
        Self::path().map_or_else(Self::default, Self::load_from)
    }

    /// The sessions in the file at `path`: none when there is no file, and
    /// none — in a store that will not save — when the file is unusable. Why
    /// it was unusable goes to the log, and to the status line through
    /// [`Self::damage`].
    #[must_use]
    pub fn load_from(path: PathBuf) -> Self {
        if path
            .file_name()
            .is_some_and(|name| name == super::yaml::USER_FILE_NAME)
        {
            return super::yaml::load(&path);
        }
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
                    backing: Backing::Damaged(reason.to_string()),
                }
            }
        }
    }

    /// Whether the file existed and did not load.
    #[must_use]
    pub const fn is_damaged(&self) -> bool {
        matches!(self.backing, Backing::Damaged(_))
    }

    /// Why the file did not load, as the error every save is refused with;
    /// `None` when it loaded or there is none.
    #[must_use]
    pub fn damage(&self) -> Option<StoreError> {
        match &self.backing {
            Backing::Damaged(reason) => Some(StoreError::Damaged(reason.clone())),
            Backing::SessionOnly | Backing::File(_) | Backing::Yaml { .. } => None,
        }
    }

    /// Write the store where it was loaded from: a sibling temp file with
    /// mode `0600` replaces the old one in a single rename. A session-only
    /// store has nowhere to go and succeeds; a damaged one refuses.
    pub fn save(&mut self) -> Result<(), StoreError> {
        let path = match &mut self.backing {
            Backing::SessionOnly => return Ok(()),
            Backing::Damaged(reason) => return Err(StoreError::Damaged(reason.clone())),
            Backing::File(path) => path,
            Backing::Yaml { path, loaded } => {
                super::yaml::save(path, loaded, &self.sessions)?;
                loaded.clone_from(&self.sessions);
                return Ok(());
            }
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
            crate::file_write::create_dirs_owner_only(dir).map_err(StoreError::Write)?;
        }
        // The settings' own guards: through a link, and never over another
        // user's file, a hard-linked one or a read-only one.
        crate::file_write::replace_owner_only(path, FILE_NAME, &text).map_err(StoreError::Write)
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
