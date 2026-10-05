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

/// Runs `suite` with the same stack budget for generated Cucumber futures on
/// Windows and Unix.
pub fn run(suite: impl FnOnce() + Send + 'static) -> io::Result<()> {
    // Windows executable entry threads have a smaller stack than libtest workers.
    // Construct and poll the large generated runner on this dedicated test thread.
    std::thread::Builder::new()
        .name("cucumber".to_owned())
        .stack_size(8 * 1024 * 1024)
        .spawn(suite)
        .map_err(|error| io::Error::new(error.kind(), format!("start BDD runner: {error}")))?
        .join()
        .unwrap_or_else(|panic| std::panic::resume_unwind(panic));
    Ok(())
}
