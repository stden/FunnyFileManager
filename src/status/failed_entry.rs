//! Purpose: `FailedEntry` — one file an operation could not process, and why.

use std::fmt;

/// One file an operation could not process, and why.
///
/// A struct rather than a pre-formatted `"path: error"` string: the two halves
/// were joined at the point of failure, which threw away the ability to show
/// them differently — a name in the status bar, the full path in a dialog, the
/// error alone in a log. The one rendering is the status line's, in
/// `status/display`, which quotes the path apart from the reason so a narrow
/// info row can shorten the path and keep both the file name and the reason.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FailedEntry {
    path: std::path::PathBuf,
    error: String,
}

impl FailedEntry {
    #[must_use]
    pub fn new(path: impl Into<std::path::PathBuf>, error: &dyn fmt::Display) -> Self {
        Self {
            path: path.into(),
            error: error.to_string(),
        }
    }

    #[must_use]
    pub fn path(&self) -> &std::path::Path {
        &self.path
    }

    #[must_use]
    pub fn error(&self) -> &str {
        &self.error
    }
}
