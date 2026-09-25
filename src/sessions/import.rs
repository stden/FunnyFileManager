//! Purpose: What an import of stored sessions found, what it passed over and
//! why, and why it produced nothing at all.
//!
//! The importer itself is [`netbox`], a pure function over the file it is
//! handed; taking its result into the store is [`super::SessionStore::adopt`],
//! and where the file is and how the outcome is reported is
//! `ops::sessions`' job.

use std::fmt;

use crate::models::InvalidField;

use super::ReadError;

pub mod netbox;

/// The longest name a skipped entry is reported under, in characters.
const MAX_SHOWN_NAME: usize = 80;

/// Why one entry of the source did not become a session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkipReason {
    /// A value failed its check.
    Invalid(InvalidField),
    /// A value is not valid `%XX` URL encoding of UTF-8 text.
    BadEncoding(&'static str),
    /// The same setting appears twice, so which one is meant is unknown.
    Repeated(&'static str),
    /// A protocol this program does not speak, such as `WebDAV` or S3.
    UnsupportedProtocol,
    /// An earlier entry of the same source already has this name.
    Duplicate,
    /// A session of that name was made or changed by hand and is kept.
    ManualEntryKept,
}

impl fmt::Display for SkipReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(field) => write!(f, "{field}"),
            Self::BadEncoding(setting) => write!(f, "{setting}: not valid URL-encoded text"),
            Self::Repeated(setting) => write!(f, "{setting} is given more than once"),
            Self::UnsupportedProtocol => {
                f.write_str("a protocol other than SFTP, SCP, FTP or FTPS")
            }
            Self::Duplicate => f.write_str("an earlier entry has the same name"),
            Self::ManualEntryKept => f.write_str("already stored and changed by hand; kept"),
        }
    }
}

impl From<InvalidField> for SkipReason {
    fn from(field: InvalidField) -> Self {
        Self::Invalid(field)
    }
}

/// An entry passed over: the name it is reported under — made safe to
/// print, since it comes from the file — and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Skipped {
    name: String,
    pub reason: SkipReason,
}

impl Skipped {
    /// `name` with every control character replaced and cut to a length a
    /// status line can hold.
    #[must_use]
    pub fn new(name: &str, reason: SkipReason) -> Self {
        let name = name
            .chars()
            .map(|ch| if ch.is_control() { '\u{FFFD}' } else { ch })
            .take(MAX_SHOWN_NAME)
            .collect();
        Self { name, reason }
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl fmt::Display for Skipped {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.name, self.reason)
    }
}

/// What an importer found: the sessions, the entries it passed over, and how
/// many saved passwords it left out — a password is never imported.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Imported {
    pub sessions: Vec<crate::models::Session>,
    pub skipped: Vec<Skipped>,
    pub passwords_left_out: usize,
}

/// What the store did with an [`Imported`]: how many it took, and every
/// entry it did not with the reason — the importer's first, then its own.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ImportReport {
    pub imported: usize,
    pub skipped: Vec<Skipped>,
    pub passwords_left_out: usize,
}

impl fmt::Display for ImportReport {
    /// The status-bar line: `3 imported, 1 skipped`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} imported, {} skipped",
            self.imported,
            self.skipped.len()
        )
    }
}

/// Why an import produced nothing at all.
#[derive(Debug)]
pub enum ImportError {
    Read(ReadError),
    /// Not well-formed XML, or XML with a DTD — which is refused outright,
    /// so no entity is ever expanded and nothing outside the file is read.
    Xml(roxmltree::Error),
    /// Well-formed XML whose root is not `<NetBox>`.
    NotNetBox,
    /// More sessions than one import takes.
    TooManySessions {
        limit: usize,
    },
    /// Elements nested deeper than an export ever is. Refused before the
    /// parser runs: it recurses once a level, and a deep enough file
    /// overflowed the stack.
    TooDeep {
        limit: usize,
    },
}

impl fmt::Display for ImportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read(error) => write!(f, "cannot read the file: {error}"),
            Self::Xml(error) => write!(f, "not a usable XML file: {error}"),
            Self::NotNetBox => f.write_str("not a NetBox session export"),
            Self::TooManySessions { limit } => {
                write!(f, "more than {limit} sessions in one file")
            }
            Self::TooDeep { limit } => {
                write!(f, "elements nested more than {limit} deep")
            }
        }
    }
}

impl std::error::Error for ImportError {}

#[cfg(test)]
mod tests;
