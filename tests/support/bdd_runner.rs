//! Runs a Cucumber suite and gives its steps one way to fail.
//!
//! A step that returns `Err` fails its scenario with the message, as a panic
//! would; clippy.toml's test allowances do not reach a `harness = false`
//! target, so the steps report failures instead of unwrapping.

use std::fmt::Debug;
use std::io;

/// What a fallible step returns: `Err` carries why the scenario failed.
pub type StepResult<T = ()> = Result<T, String>;

/// Turns an error or a missing value into a step failure that reads exactly
/// like the panic of the `expect` it replaces.
pub trait Context<T> {
    fn context(self, doing: &str) -> StepResult<T>;
}

impl<T, E: Debug> Context<T> for Result<T, E> {
    /// `Result::expect` panics with `{msg}: {error:?}`, so this does too.
    fn context(self, doing: &str) -> StepResult<T> {
        self.map_err(|error| format!("{doing}: {error:?}"))
    }
}

impl<T> Context<T> for Option<T> {
    fn context(self, doing: &str) -> StepResult<T> {
        self.ok_or_else(|| doing.to_owned())
    }
}

/// The text of a failed step, or an empty string when the step succeeded.
#[must_use]
pub fn step_error(result: StepResult) -> String {
    match result {
        Err(message) => message,
        Ok(()) => String::new(),
    }
}

/// Fails the step unless `result` is an error whose text contains `needle`.
pub fn expect_error(result: StepResult, needle: &str) {
    let message = step_error(result);
    assert!(message.contains(needle), "{message}");
}

fn start_error(error: io::Error) -> io::Error {
    let kind = error.kind();
    let detail = error.to_string();
    drop(error);
    io::Error::new(kind, format!("start BDD runner: {detail}"))
}

fn rethrow(panic: Box<dyn std::any::Any + Send>) {
    std::panic::resume_unwind(panic)
}

/// Runs the branches a green suite never takes: a missing value, a thread
/// that could not start, and a panicked runner.
pub fn cover_error_paths() {
    assert!(step_error(Ok(())).is_empty());
    assert_eq!(step_error(Err("marker".to_owned())), "marker");
    let missing = None::<u8>.context("missing");
    assert!(matches!(missing, Err(message) if message == "missing"));
    drop(start_error(io::Error::other("spawn")));
    let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        rethrow(Box::new("bdd runner panic".to_owned()));
    }));
    assert!(caught.is_err());
}

/// Runs `suite` with the same stack budget for generated Cucumber futures on
/// Windows and Unix.
pub fn run(suite: impl FnOnce() + Send + 'static) -> io::Result<()> {
    // Windows executable entry threads have a smaller stack than libtest workers.
    // Construct and poll the large generated runner on this dedicated test thread.
    std::thread::Builder::new()
        .name("cucumber".to_owned())
        .stack_size(8 * 1024 * 1024)
        .spawn(suite)
        .map_err(start_error)?
        .join()
        .unwrap_or_else(rethrow);
    Ok(())
}
