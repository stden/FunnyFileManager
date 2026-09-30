//! Purpose: "Retry as administrator": the copy, move or delete Windows
//! refused, done by a helper FFM starts as an administrator — Windows asks
//! once (UAC) — and keeps for the rest of the session, so every later
//! refusal is retried by it without asking again.
//!
//! The helper is this same program, started with [`HELPER_FLAG`]. It talks
//! to FFM over a local pipe: FFM sends what to redo, the helper sends back
//! its progress and then its outcome. When FFM exits, the pipe closes and
//! the helper ends with it.

use std::io;

use crate::models::{App, ElevatedRetry, OperationFailed, Overlay};
use crate::progress::Operation;
use crate::status::{FailedEntry, Landed, Status};

// The helper runs only on Windows; the tests drive it on every platform.
#[cfg(any(windows, test))]
mod helper;
mod protocol;
#[cfg(windows)]
mod session;

use protocol::Outcome;

/// The command-line flag that makes FFM the administrator helper.
pub const HELPER_FLAG: &str = "--elevated-helper";

/// Windows' answer when the user says no to the UAC prompt.
const CANCELLED: i32 = 1223;

/// Whether retrying as an administrator is possible here at all: Windows,
/// and FFM not already running as one.
#[must_use]
pub fn available() -> bool {
    #[cfg(windows)]
    {
        !crate::platform::is_elevated()
    }
    #[cfg(not(windows))]
    {
        false
    }
}

/// Whether the administrator helper already runs for this session, so a
/// refusal can be retried without asking.
#[must_use]
pub fn session_active() -> bool {
    #[cfg(windows)]
    {
        session::active()
    }
    #[cfg(not(windows))]
    {
        false
    }
}

/// Run FFM as the administrator helper when `args` say so, until FFM closes
/// the pipe. `None` for any other command line.
#[must_use]
pub fn helper_main(args: &[std::ffi::OsString]) -> Option<io::Result<()>> {
    let [flag, name, parent] = args else {
        return None;
    };
    if flag != HELPER_FLAG {
        return None;
    }
    #[cfg(windows)]
    let served = session::serve_parent(name, parent);
    #[cfg(not(windows))]
    let served: io::Result<()> = {
        let _ = (name, parent);
        Err(io::Error::from(io::ErrorKind::Unsupported))
    };
    Some(served)
}

/// The dialog's "Retry as administrator": ask Windows for the rights, then
/// redo what failed and what was not started.
pub fn retry_as_administrator(app: &mut App) {
    let Overlay::OperationFailed(failure) = &mut app.overlay else {
        return;
    };
    let Some(retry) = failure.take_retry() else {
        return;
    };
    let done = failure.done();
    app.close_overlay();
    run(app, done, &retry);
}

/// Redo `retry` as an administrator — through the session's helper, started
/// first if there is none — and report the whole operation: `done` entries
/// went before it stopped.
pub(super) fn run(app: &mut App, done: usize, retry: &ElevatedRetry) {
    match exchange(retry) {
        Ok(outcome) => finish(app, done, retry, &outcome),
        Err(error) if error.raw_os_error() == Some(CANCELLED) => {
            app.set_status(Status::Error(
                "Administrator rights were not granted".to_owned(),
            ));
        }
        Err(error) => app.set_status(Status::Error(format!(
            "Could not run as administrator: {error}"
        ))),
    }
    super::refresh_after_operation(app);
}

#[cfg(windows)]
fn exchange(retry: &ElevatedRetry) -> io::Result<Outcome> {
    session::exchange(retry)
}

#[cfg(not(windows))]
fn exchange(_retry: &ElevatedRetry) -> io::Result<Outcome> {
    Err(io::Error::from(io::ErrorKind::Unsupported))
}

/// Report what the administrator did, on top of the `done` entries before.
fn finish(app: &mut App, done: usize, retry: &ElevatedRetry, outcome: &Outcome) {
    let count = done + outcome.done;
    let purpose = retry.settings.as_ref().map(|settings| settings.purpose);
    let Some(failure) = &outcome.failed else {
        app.set_status(match purpose {
            None => Status::Deleted { count },
            Some(purpose) => Status::Transferred {
                purpose,
                landed: Landed::of(count, None),
                skipped: outcome.skipped,
                renamed: outcome.renamed,
            },
        });
        return;
    };
    let failed = FailedEntry::new(&failure.path, &failure.error);
    app.set_status(purpose.map_or_else(
        || Status::DeletedWithErrors {
            count,
            error_count: 1,
            first_error: failed.clone(),
        },
        |purpose| super::transfer::stopped_status(purpose, count, 1, &failed),
    ));
    // Nothing more to offer: this is what an administrator could not do.
    let stopped = OperationFailed::new(retry.operation, failed, count, outcome.left)
        .because(failure.denial.clone());
    app.set_overlay(Overlay::OperationFailed(Box::new(stopped)));
}

/// What a stopped operation is retried as, when an administrator might get
/// past `denial` and one can be asked for: only when `allowed`, which is the
/// live session's say (`LiveInput::elevation`), never a test's or a replay's.
pub(super) fn retry_for(
    allowed: bool,
    operation: Operation,
    denial: Option<&crate::models::Denial>,
    remaining: &[std::path::PathBuf],
    base: &std::path::Path,
    selected: usize,
    settings: Option<&crate::copy::CopyPrompt>,
) -> Option<ElevatedRetry> {
    let may_help = denial.is_some_and(super::denied::administrator_may_help);
    // The quick F5/F6 has no dialog settings to send; only F8 goes without.
    let complete = settings.is_some() == (operation != Operation::Delete);
    let possible = allowed && may_help && complete && !remaining.is_empty();
    (possible && available()).then(|| ElevatedRetry {
        operation,
        sources: remaining.to_vec(),
        base: base.to_path_buf(),
        selected,
        settings: settings.cloned(),
    })
}

#[cfg(test)]
mod tests;
