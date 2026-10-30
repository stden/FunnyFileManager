//! Purpose: a failure that names the path it happened at — the file a copy
//! would overwrite, the folder it would write into, the folder F3 or Enter
//! could not read — so the status line can say which.

use std::{
    fmt, io,
    path::{Path, PathBuf},
};

/// `error`, which happened at `path`.
///
/// Carried inside an `io::Error`, like `recording::TerminalLost`, so it
/// passes through every `?` up to whoever reports it, which names `path`: a
/// copy's batch rather than the source — "…/src2/file.txt: Permission
/// denied" sent the user to a file that was fine, when the folder it was
/// copied into was read-only — and the status bar rather than a bare
/// "Error: Permission denied". Its own text is the error's, unchanged.
#[derive(Debug)]
pub struct FailedAt {
    path: PathBuf,
    error: io::Error,
}

impl fmt::Display for FailedAt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.error.fmt(f)
    }
}

impl std::error::Error for FailedAt {}

impl FailedAt {
    /// `error` as a failure at `path`, keeping its kind. An error that
    /// already names where it happened keeps that place.
    #[must_use]
    pub fn wrap(path: &Path, error: io::Error) -> io::Error {
        if Self::of(&error).is_some() {
            return error;
        }
        io::Error::new(
            error.kind(),
            Self {
                path: path.to_path_buf(),
                error,
            },
        )
    }

    /// The `FailedAt` `error` carries, if it carries one.
    #[must_use]
    pub fn of(error: &io::Error) -> Option<&Self> {
        error.get_ref()?.downcast_ref()
    }

    /// Where it happened.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}
