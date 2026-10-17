//! Process diagnostics: on by default in the file manager, off in tests unless
//! asked for. Logging never changes application outcomes.

use std::error::Error;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

use crossterm::event::KeyCode;

use crate::action::Action;

/// Whether a run with no `FFM_LOG` logs: the file manager itself does (see
/// [`log_by_default`]); a test or a tool built on the library does not.
static ON_BY_DEFAULT: AtomicBool = AtomicBool::new(false);

/// Log unless `FFM_LOG=0` says not to. The production entry point calls this
/// before anything is logged; tests never do, so a test run writes no log
/// into the temporary directory unless `FFM_LOG=1` asks it to.
pub fn log_by_default() {
    ON_BY_DEFAULT.store(true, Ordering::Relaxed);
}

/// Whether to log, given `FFM_LOG`: `1` turns it on, `0` off, and anything
/// else — unset included — leaves it at `by_default`.
fn enabled(setting: Option<&str>, by_default: bool) -> bool {
    match setting {
        Some("1") => true,
        Some("0") => false,
        _ => by_default,
    }
}

/// Directory the logs are written to, or `None` when logging is off.
///
/// The file manager logs by default and `FFM_LOG=0` turns it off; elsewhere
/// it is off unless `FFM_LOG=1`. Key and action labels redact typed
/// characters, but diagnostics may still describe user paths. Logs use the
/// OS temporary directory rather than the working directory. The setting is
/// read once and stays fixed for the process lifetime.
fn log_dir() -> Option<&'static Path> {
    static DIR: OnceLock<Option<PathBuf>> = OnceLock::new();
    DIR.get_or_init(|| {
        let setting = std::env::var("FFM_LOG").ok();
        enabled(setting.as_deref(), ON_BY_DEFAULT.load(Ordering::Relaxed)).then(std::env::temp_dir)
    })
    .as_deref()
}

/// Append one timestamped line to `path`, creating it owner-only where the OS
/// supports it. Failures are ignored on purpose: logging must never be able to
/// break the file manager.
fn append_log_line(path: &Path, line: &str) {
    let Some(mut file) = open_log_file(path) else {
        return;
    };
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis());
    // A failed write is ignored on purpose, as the doc comment says.
    drop(writeln!(file, "[{timestamp}] {line}"));
}

fn open_log_file(path: &Path) -> Option<std::fs::File> {
    if let Ok(meta) = std::fs::symlink_metadata(path) {
        if meta.file_type().is_symlink() || !meta.is_file() || !crate::file_write::owned_by_us(path)
        {
            return None;
        }
    }
    let mut options = OpenOptions::new();
    options.create(true).append(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let file = crate::fileload::open_regular(path, &options).ok()?;
    let meta = file.metadata().ok()?;
    if crate::directory::link_count(&meta) > 1 || !crate::file_write::owned_by_us(path) {
        return None;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        drop(file.set_permissions(std::fs::Permissions::from_mode(0o600)));
    }
    Some(file)
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
