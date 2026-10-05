//! Purpose: `Landed` — where a copy or move that went through put what it transferred.

use std::path::PathBuf;

/// Where a copy or move put what it transferred. The status line names the
/// place only when one entry went, and counts the entries otherwise.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Landed {
    /// Nothing went: the F5 / F6 dialog's choice left every entry alone.
    Nowhere,
    /// One entry, now at this path.
    At(PathBuf),
    /// More than one entry.
    Many(usize),
}

impl Landed {
    /// `count` entries transferred, the first of them to `first`.
    #[must_use]
    pub fn of(count: usize, first: Option<PathBuf>) -> Self {
        match first {
            _ if count > 1 => Self::Many(count),
            Some(path) => Self::At(path),
            None => Self::Nowhere,
        }
    }
}
