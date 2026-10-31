//! Purpose: User server profiles in ffm.yml, legacy JSON compatibility, and imports.
//!
//! Written the way the history is: atomically, readable by the owner alone.
//! No password is ever in it; see [`crate::models::Session`].
//!
//! Loading never fails. No file is an empty store. A file that exists but
//! cannot be read, is too large, does not parse, or holds a value a session
//! may not have is an empty store that will not save — one bad byte, a hand
//! edit or a file from a newer build must not be overwritten with nothing by
//! the next change — and says why on the status line. `App::new()` never loads it; `bootstrap` does, so no test
//! reads the user's own sessions.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::models::{Session, SessionName};

mod file;
pub mod import;
mod read;
mod yaml;

pub use file::StoreError;
use import::{ImportReport, Imported, SkipReason, Skipped};
pub use read::{read_text, ReadError};

/// The file's name under the `ffm` config directory.
pub const FILE_NAME: &str = "sessions.json";
/// The largest sessions file read or written, in bytes: room for thousands of sessions.
pub const MAX_FILE_BYTES: u64 = 1024 * 1024;

/// Where the store is kept.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
enum Backing {
    /// Nowhere: a test's store, or a machine with no config directory.
    #[default]
    SessionOnly,
    File(PathBuf),
    Yaml {
        path: PathBuf,
        loaded: Vec<Session>,
    },
    /// The file exists and did not load, for this reason; nothing is
    /// written over it.
    Damaged(String),
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
    /// Add `session`, or replace the one stored under its name.
    pub fn upsert(&mut self, session: Session) {
        match self
            .sessions
            .iter_mut()
            .find(|kept| kept.name == session.name)
        {
            Some(kept) => *kept = session,
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

#[cfg(test)]
mod tests;
