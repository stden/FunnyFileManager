//! Purpose: Everything that waits for the terminal to be the user's again:
//! the state written back to disk, the lines left on stderr, and the outcome
//! handed to `main`.

use std::error::Error;
use std::io;

use crate::models::App;

/// Report how the session ended, and hand `main` its outcome.
///
/// A terminal that hung up can be neither drawn on nor restored, so the errors
/// that say so are how the session ended, not a failure of it: the loop quits
/// on a hang-up the way F10 asks. Reporting them would also write to the dead
/// terminal, and `eprintln!` panics when that write fails.
pub(super) fn end_session(
    app: &App,
    run_result: io::Result<()>,
    cleanup_result: io::Result<()>,
) -> Result<(), Box<dyn Error>> {
    if crate::recording::wait_for_terminal(std::time::Duration::ZERO).is_err() {
        report_at_exit(app, &Ok(()));
        return Ok(());
    }
    report_at_exit(app, &run_result);
    crate::terminal::merge_run_and_cleanup(run_result, cleanup_result).map_err(Into::into)
}

/// Everything that has to wait for the screen to be the user's again: the
/// state written back to disk, a recording that gave up halfway, and the
/// failure that ended the session, if one did.
#[expect(
    clippy::print_stderr,
    reason = "runs after the terminal is restored: stderr is the only channel left for a failure"
)]
fn report_at_exit(app: &App, run_result: &io::Result<()>) {
    persist_at_exit(app);
    if let Err(error) = app.performance.save_session() {
        eprintln!("ffm: could not save action statistics: {error}");
    }
    if let Some(reason) = crate::recording::stopped_reason() {
        eprintln!("ffm: recording stopped: {reason}");
    }
    if let Err(error) = run_result {
        eprintln!("{}", AtExitErrorLine(error));
    }
}

/// The line a failed session leaves on stderr: the `io::Error` in its Debug
/// form, which also names the `ErrorKind`. `main` prints the Display form
/// after it.
pub(super) struct AtExitErrorLine<'error>(pub(super) &'error io::Error);

impl std::fmt::Display for AtExitErrorLine<'_> {
    #[expect(
        clippy::use_debug,
        reason = "the exit line deliberately shows the io::Error's Debug form, which names its ErrorKind; main prints the Display form after it"
    )]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Error: {:?}", self.0)
    }
}

/// Write the histories, and the settings when "Auto save setup" is on, after
/// the terminal is back in the user's hands. Failures are logged, never
/// raised: a full disk must not turn a clean exit into an error report.
pub(super) fn persist_at_exit(app: &App) {
    let mut writes = vec![
        ("view history", app.view_history.save()),
        ("folder history", app.folder_history.save()),
    ];
    if app.config.system.auto_save_setup {
        writes.push(("settings", app.config.save()));
    }
    for (what, result) in writes {
        if let Err(error) = result {
            crate::diagnostics::log_error(&format!("saving {what}: {error}"));
        }
    }
}
