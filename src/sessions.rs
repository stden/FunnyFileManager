//! Purpose: The stored sessions file — `NetBox`'s session list as one JSON
//! file beside the settings — and taking in what an import found.
//!
//! Written the way the history is: atomically, readable by the owner alone.
//! No password is ever in it; see [`crate::models::Session`].
//!
//! Loading never fails. No file is an empty store. A file that exists but
//! cannot be read, is too large, does not parse, or holds a value a session
//! may not have is an empty store that will not save — one bad byte, a hand
//! edit or a file from a newer build must not be overwritten with nothing by
//! the next change. `App::new()` never loads it; `bootstrap` does, so no test
//! reads the user's own sessions.

use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::fileload::{read_regular, Loaded};
use crate::models::{Session, SessionName};

pub mod import;

use import::{ImportReport, Imported, SkipReason, Skipped};

/// The file's name under the `ffm` config directory.
pub const FILE_NAME: &str = "sessions.json";
/// The largest sessions file read, in bytes: room for thousands of sessions.
pub const MAX_FILE_BYTES: u64 = 1024 * 1024;

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
pub fn read_text(path: &Path, limit: u64) -> Result<String, ReadError> {
    match read_regular(path, limit).map_err(ReadError::Io)? {
        Loaded::Bytes(bytes) => String::from_utf8(bytes).map_err(|_| ReadError::NotUtf8),
        Loaded::NotRegular => Err(ReadError::NotAFile),
        Loaded::TooLarge(_) => Err(ReadError::TooLarge { limit }),
    }
}

/// Why the store was not written.
#[derive(Debug)]
pub enum StoreError {
    /// The file did not load at start-up, so it is kept as it is.
    Damaged,
    Write(io::Error),
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Damaged => f.write_str(
                "the sessions file did not load, so it is not overwritten; fix or move it first",
            ),
            Self::Write(error) => write!(f, "cannot write the sessions file: {error}"),
        }
    }
}

/// Where the store is kept.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
enum Backing {
    /// Nowhere: a test's store, or a machine with no config directory.
    #[default]
    SessionOnly,
    File(PathBuf),
    /// This file exists and did not load; nothing is written over it.
    Damaged(PathBuf),
}

/// The file's shape: an object, so a later field has somewhere to go.
#[derive(Serialize, Deserialize)]
struct StoredFile {
    sessions: Vec<Session>,
}

/// Every stored session, in the order they were added; [`Self::sorted`] is
/// the display order. Names are unique.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SessionStore {
    sessions: Vec<Session>,
    backing: Backing,
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
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(StoreError::Write)?;
        }
        crate::history::write_owner_only(path, &text).map_err(StoreError::Write)
    }

    /// Add `session`, or replace the one stored under its name.
    pub fn upsert(&mut self, session: Session) {
        match self.position(&session.name) {
            Some(at) => self.sessions[at] = session,
            None => self.sessions.push(session),
        }
    }

    /// Remove the session named `name`, handing it back if there was one.
    pub fn remove(&mut self, name: &SessionName) -> Option<Session> {
        self.position(name).map(|at| self.sessions.remove(at))
    }

    #[must_use]
    pub fn find(&self, name: &SessionName) -> Option<&Session> {
        self.sessions.iter().find(|session| &session.name == name)
    }

    /// Every session, by name — the panel's order.
    #[must_use]
    pub fn sorted(&self) -> Vec<&Session> {
        let mut sessions: Vec<&Session> = self.sessions.iter().collect();
        sessions.sort_by(|a, b| a.name.cmp(&b.name));
        sessions
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.sessions.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.sessions.is_empty()
    }

    /// Take in what an import found. A name the user made or changed by hand
    /// is kept and reported as skipped; one an earlier import wrote is
    /// refreshed.
    pub fn adopt(&mut self, imported: Imported) -> ImportReport {
        let mut report = ImportReport {
            imported: 0,
            skipped: imported.skipped,
            passwords_left_out: imported.passwords_left_out,
        };
        for session in imported.sessions {
            if self
                .find(&session.name)
                .is_some_and(|kept| kept.origin.is_manual())
            {
                report.skipped.push(Skipped::new(
                    session.name.as_str(),
                    SkipReason::ManualEntryKept,
                ));
                continue;
            }
            self.upsert(session);
            report.imported += 1;
        }
        report
    }

    fn position(&self, name: &SessionName) -> Option<usize> {
        self.sessions
            .iter()
            .position(|session| &session.name == name)
    }
}

/// Why a sessions file did not load; for the log.
#[derive(Debug)]
enum LoadError {
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
fn read_sessions(path: &Path) -> Result<Vec<Session>, LoadError> {
    let text = match read_text(path, MAX_FILE_BYTES) {
        Ok(text) => text,
        Err(ReadError::Io(error)) if error.kind() == io::ErrorKind::NotFound => {
            return Ok(Vec::new())
        }
        Err(error) => return Err(LoadError::Read(error)),
    };
    let file: StoredFile = serde_json::from_str(&text).map_err(LoadError::Parse)?;
    let mut names: Vec<&SessionName> = file.sessions.iter().map(|s| &s.name).collect();
    names.sort_unstable();
    if names.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(LoadError::DuplicateName);
    }
    Ok(file.sessions)
}

#[cfg(test)]
mod tests;
