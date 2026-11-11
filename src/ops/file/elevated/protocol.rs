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

/// What FFM sends the helper for one retry: the `ElevatedRetry` itself and
/// any per-file "already existing files" answers from the clash dialog.
#[cfg_attr(
    not(any(windows, test)),
    expect(
        dead_code,
        reason = "only Windows and tests construct and send helper requests"
    )
)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Request {
    #[serde(flatten)]
    pub retry: crate::models::ElevatedRetry,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub choices: Vec<(PathBuf, crate::copy::ExistingFiles)>,
}

#[derive(Default)]
struct RetryContext {
    #[cfg_attr(
        not(any(windows, test)),
        expect(
            dead_code,
            reason = "only Windows and tests read the saved retry choices"
        )
    )]
    choices: Vec<(PathBuf, crate::copy::ExistingFiles)>,
    skipped: usize,
    renamed: usize,
}

thread_local! {
    static CONTEXT: std::cell::RefCell<RetryContext> =
        std::cell::RefCell::new(RetryContext::default());
}

pub(in super::super) fn remember_context(
    choices: &[(PathBuf, crate::copy::ExistingFiles)],
    skipped: usize,
    renamed: usize,
) {
    CONTEXT.with(|ctx| {
        *ctx.borrow_mut() = RetryContext {
            choices: choices.to_vec(),
            skipped,
            renamed,
        };
    });
}

#[cfg(any(windows, test))]
pub(super) fn context_choices() -> Vec<(PathBuf, crate::copy::ExistingFiles)> {
    CONTEXT.with(|ctx| ctx.borrow().choices.clone())
}

pub(super) fn take_prior_counts() -> (usize, usize) {
    CONTEXT.with(|ctx| {
        let mut ctx = ctx.borrow_mut();
        let counts = (ctx.skipped, ctx.renamed);
        ctx.skipped = 0;
        ctx.renamed = 0;
        counts
    })
}

/// The pipe itself: only Windows runs the helper, and the tests speak it
/// over in-memory buffers. Elsewhere FFM never starts a helper, so nothing
/// here would be called.
#[cfg(any(windows, test))]
mod wire {
    use std::io::{self, BufRead, Read as _, Write};

    use serde::{de::DeserializeOwned, Deserialize, Serialize};

    use super::Outcome;
    use crate::progress::Snapshot;

    /// Upper bound on one JSON line over the helper pipe (10 MiB).
    const MAX_LINE_BYTES: u64 = 10 * 1024 * 1024;

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
        if input.take(MAX_LINE_BYTES).read_line(&mut line)? == 0 {
            return Ok(None);
        }
        serde_json::from_str(&line)
            .map(Some)
            .map_err(io::Error::other)
    }
}
