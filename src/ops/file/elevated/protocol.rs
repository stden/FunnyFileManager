//! Purpose: What FFM and its administrator helper say to each other: one
//! JSON document a line. FFM sends an `ElevatedRetry`; the helper answers
//! with progress reports, then the outcome.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::models::Denial;

#[cfg(any(windows, test))]
pub(super) use wire::{receive, send, Reply};

/// What the helper did with one request.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Outcome {
    /// Entries done.
    pub done: usize,
    /// Entries not started because one failed.
    pub left: usize,
    /// Entries the "already existing files" choice left alone.
    pub skipped: usize,
    /// Entries put beside an existing one under a new name.
    pub renamed: usize,
    /// Where it stopped, if it did.
    pub failed: Option<Failure>,
}

/// Where and why the helper stopped.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Failure {
    pub path: PathBuf,
    pub error: String,
    pub denial: Option<Denial>,
}

/// The pipe itself: only Windows runs the helper, and the tests speak it
/// over in-memory buffers. Elsewhere FFM never starts a helper, so nothing
/// here would be called.
#[cfg(any(windows, test))]
mod wire {
    use std::io::{self, BufRead, Write};

    use serde::{de::DeserializeOwned, Deserialize, Serialize};

    use super::Outcome;
    use crate::progress::Snapshot;

    /// One line from the helper.
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub enum Reply {
        Progress(Snapshot),
        Finished(Outcome),
    }

    /// Write `message` as one line and flush it.
    pub(in super::super) fn send(out: &mut impl Write, message: &impl Serialize) -> io::Result<()> {
        serde_json::to_writer(&mut *out, message)?;
        out.write_all(b"\n")?;
        out.flush()
    }

    /// Read the next line as a `T`; `None` once the other end has closed.
    pub(in super::super) fn receive<T: DeserializeOwned>(
        input: &mut impl BufRead,
    ) -> io::Result<Option<T>> {
        let mut line = String::new();
        if input.read_line(&mut line)? == 0 {
            return Ok(None);
        }
        serde_json::from_str(&line)
            .map(Some)
            .map_err(io::Error::other)
    }
}
