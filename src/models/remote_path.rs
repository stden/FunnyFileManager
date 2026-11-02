//! Purpose: `RemotePath` — an absolute, `/`-separated path on a remote host,
//! normalised once so every holder can compare, join and climb it safely.
//!
//! A newtype because a remote path is not a local `Path`: it always uses
//! `/`, whatever this program runs on, and its parts come from two places
//! that must both be checked — what a user types or a file imports (the
//! session's start directory), and the names a host lists. A name holding
//! `/` or spelled `..` would walk the panel somewhere it did not show, so
//! [`RemotePath::join`] refuses them rather than trusting the host.

use std::fmt;

use serde::{Deserialize, Serialize};

use super::{Field, InvalidField, Problem};

/// The longest path accepted from a user or an imported file, in bytes —
/// Linux's `PATH_MAX`.
pub const MAX_PATH_BYTES: usize = 4096;

/// An absolute remote path: `/`, or `/` followed by names joined with `/`,
/// none of them empty, `.` or `..`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct RemotePath(String);

impl RemotePath {
    /// `/`.
    #[must_use]
    pub fn root() -> Self {
        Self("/".to_owned())
    }

    /// A start directory as typed or imported: absolute, at most
    /// [`MAX_PATH_BYTES`], no control characters, no `.` or `..` parts.
    /// Surrounding blanks are dropped and repeated or trailing `/` collapse,
    /// so `/var//www/` is `/var/www`.
    pub fn parse(text: &str) -> Result<Self, InvalidField> {
        let text = text.trim();
        let fail = |problem| InvalidField::new(Field::RemoteDir, problem);
        if text.is_empty() {
            return Err(fail(Problem::Empty));
        }
        if text.len() > MAX_PATH_BYTES {
            return Err(fail(Problem::TooLong {
                limit: MAX_PATH_BYTES,
            }));
        }
        if text.chars().any(char::is_control) {
            return Err(fail(Problem::ControlCharacter));
        }
        if !text.starts_with('/') {
            return Err(fail(Problem::NotAbsolute));
        }
        let mut path = Self::root();
        for part in text.split('/').filter(|part| !part.is_empty()) {
            path = path.join(part).ok_or_else(|| fail(Problem::DotSegment))?;
        }
        Ok(path)
    }

    /// `name` inside this directory, or `None` when `name` is not one name:
    /// empty, `.`, `..`, or holding `/` or NUL.
    #[must_use]
    pub fn join(&self, name: &str) -> Option<Self> {
        if !is_name(name) {
            return None;
        }
        Some(if self.is_root() {
            Self(format!("/{name}"))
        } else {
            Self(format!("{}/{name}", self.0))
        })
    }

    /// The directory above, or `None` at the root.
    #[must_use]
    pub fn parent(&self) -> Option<Self> {
        if self.is_root() {
            return None;
        }
        // Every path but the root has a `/` before its last name.
        match self.0.rsplit_once('/')? {
            ("", _) => Some(Self::root()),
            (above, _) => Some(Self(above.to_owned())),
        }
    }

    #[must_use]
    pub fn is_root(&self) -> bool {
        self.0 == "/"
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Whether `name` can be one part of a remote path.
#[must_use]
pub fn is_name(name: &str) -> bool {
    !matches!(name, "" | "." | "..") && !name.contains(['/', '\0'])
}

impl fmt::Display for RemotePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl TryFrom<String> for RemotePath {
    type Error = InvalidField;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        Self::parse(&text)
    }
}

impl From<RemotePath> for String {
    fn from(path: RemotePath) -> Self {
        path.0
    }
}

#[cfg(test)]
mod tests;
