//! Process diagnostics: on by default in the file manager, off in tests unless
//! asked for. Logging never changes application outcomes.

use std::error::Error;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

use crossterm::event::KeyCode;

mod interaction;
mod queue;
pub use interaction::Interaction;

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
#[cfg(test)]
fn append_log_line_at(path: &Path, line: &str, captured: std::time::SystemTime) {
    let Some(mut file) = open_log_file(path) else {
        return;
    };
    write_log_line(&mut file, line, captured);
}

fn write_log_line(writer: &mut impl Write, line: &str, captured: std::time::SystemTime) {
    let timestamp = captured
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis());
    // A failed write is ignored on purpose, as the doc comment says.
    drop(writeln!(writer, "[{timestamp}] {line}"));
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
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let meta = file.metadata().ok()?;
        if crate::directory::link_count(&meta) > 1 || !crate::file_write::owned_by_us(path) {
            return None;
        }
        drop(file.set_permissions(std::fs::Permissions::from_mode(0o600)));
    }
    Some(file)
}

pub fn log_action(action: &str) {
    if let Some(dir) = log_dir() {
        queue::submit(dir, queue::Destination::Action, action);
    }
}

pub fn log_error(error: &str) {
    if let Some(dir) = log_dir() {
        queue::submit(dir, queue::Destination::Error, &format!("ERROR: {error}"));
    }
}

/// Describe a key for the log *without* its payload.
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
    let result = run_with_logger(main_runner, log_action, log_error);
    queue::flush();
    result
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
