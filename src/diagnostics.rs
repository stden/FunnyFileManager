//! Opt-in process diagnostics. Logging never changes application outcomes.

use std::error::Error;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use crossterm::event::KeyCode;

use crate::action::Action;

/// Directory the logs are written to, or `None` when logging is off.
///
/// Logging is opt-in (`FFM_LOG=1`) and off by default. Key and action labels
/// redact typed characters, but diagnostics may still describe user paths.
/// Logs use the OS temporary directory rather than the working directory.
/// The opt-in setting is read once and stays fixed for the process lifetime.
fn log_dir() -> Option<&'static Path> {
    static DIR: OnceLock<Option<PathBuf>> = OnceLock::new();
    DIR.get_or_init(|| {
        std::env::var("FFM_LOG")
            .is_ok_and(|value| value == "1")
            .then(std::env::temp_dir)
    })
    .as_deref()
}

/// Append one timestamped line to `path`, creating it owner-only where the OS
/// supports it. Failures are ignored on purpose: logging must never be able to
/// break the file manager.
fn append_log_line(path: &Path, line: &str) {
    let mut options = OpenOptions::new();
    options.create(true).append(true);
    #[cfg(unix)]
    {
        // 0600 — the log records the paths of whoever ran ffm; the default umask
        // would leave it readable by every other local user.
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    // A FIFO planted where the log goes is refused, never waited on.
    let Ok(mut file) = crate::fileload::open_regular(path, &options) else {
        return;
    };
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis());
    // A failed write is ignored on purpose, as the doc comment says.
    drop(writeln!(file, "[{timestamp}] {line}"));
}

fn log_action_to(path: &Path, action: &str) {
    append_log_line(path, action);
}

fn log_error_to(path: &Path, error: &str) {
    append_log_line(path, &format!("ERROR: {error}"));
}

pub fn log_action(action: &str) {
    if let Some(dir) = log_dir() {
        log_action_to(&dir.join("ffm_actions.log"), action);
    }
}

pub fn log_error(error: &str) {
    if let Some(dir) = log_dir() {
        log_error_to(&dir.join("ffm_errors.log"), error);
    }
}

/// Describe an action for the log *without* any text the user typed.
///
/// Both `QuickSearch` and `CommandLineChar` carry a character from the keyboard.
/// A derived `Debug` would expose that input. The same rule applies to
/// [`key_label`]; keeping labels pure lets tests prove the redaction.
pub fn action_label(action: Action) -> String {
    match action {
        Action::QuickSearch(_) => "QuickSearch(..)".to_owned(),
        Action::CommandLineChar(_) => "CommandLineChar(..)".to_owned(),
        other => format!("{other:?}"),
    }
}

/// Describe a key for the log *without* its payload.
///
/// `KeyCode::Char` is rendered payload-free on purpose: keys are logged before
/// an overlay gets to consume them, so printing the character would write every
/// password typed into the built-in editor straight to the log file.
pub fn key_label(code: KeyCode) -> String {
    match code {
        KeyCode::Char(_) => "Char".to_owned(),
        other => format!("{other:?}"),
    }
}

pub fn run_with<F>(main_runner: F) -> Result<(), Box<dyn Error>>
where
    F: FnOnce() -> Result<(), Box<dyn Error>>,
{
    run_with_logger(main_runner, log_action, log_error)
}

fn run_with_logger<F>(
    main_runner: F,
    action: impl Fn(&str),
    error: impl Fn(&str),
) -> Result<(), Box<dyn Error>>
where
    F: FnOnce() -> Result<(), Box<dyn Error>>,
{
    action("=== FunnyFileManager Started ===");
    let result = main_runner();
    if let Err(ref failure) = result {
        error(&format!("Main error: {failure}"));
    }
    action("=== FunnyFileManager Ended ===");
    result
}

#[cfg(test)]
mod tests;
