//! Purpose: the fish engine — spawn `complete --do-complete` for a `cd` line, bounded by a timeout, and parse what it prints.

use std::fs::File;
use std::io::{ErrorKind, Read, Seek};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// Bound on a fish completion spawn so a hung fish cannot freeze the TUI.
const FISH_COMPLETE_TIMEOUT: Duration = Duration::from_millis(1500);
const POLL_START: Duration = Duration::from_micros(250);
const POLL_MAX: Duration = Duration::from_millis(10);

/// Maximum completion bytes read back from fish stdout spool.
const MAX_FISH_OUTPUT: u64 = 1024 * 1024;

/// Set once `Command::new("fish")` reports `ErrorKind::NotFound` so subsequent
/// `Tab` presses skip `tempfile::tempfile()` and process spawn overhead.
static FISH_UNAVAILABLE: AtomicBool = AtomicBool::new(false);

#[cfg(any(test, tarpaulin))]
thread_local! {
    pub(crate) static FISH_PROGRAM_FOR_TESTS: std::cell::RefCell<Option<PathBuf>> =
        const { std::cell::RefCell::new(None) };
}

#[cfg(any(test, tarpaulin))]
pub(super) fn reset_fish_unavailable() {
    FISH_UNAVAILABLE.store(false, Ordering::Relaxed);
}

struct FishProgram {
    binary: PathBuf,
    memoize_missing: bool,
}

/// The program spoken to as fish and whether `FISH_UNAVAILABLE` applies.
fn fish_program() -> FishProgram {
    #[cfg(any(test, tarpaulin))]
    if let Some(binary) = FISH_PROGRAM_FOR_TESTS.with(|slot| slot.borrow().clone()) {
        return FishProgram {
            binary,
            memoize_missing: false,
        };
    }
    FishProgram {
        binary: PathBuf::from("fish"),
        memoize_missing: true,
    }
}

/// Ask the fish shell for `cd <token>` completions, run with `cwd` as the
/// working directory. Returns `None` when fish is unavailable, times out, or
/// yields nothing, so the caller can fall back.
pub(super) fn fish_path_complete(token: &str, cwd: &Path) -> Option<Vec<String>> {
    let program = fish_program();
    if program.memoize_missing && FISH_UNAVAILABLE.load(Ordering::Relaxed) {
        return None;
    }
    let line = format!("cd {token}");
    let stdout = tempfile::tempfile().ok()?;
    let stdout_writer = stdout.try_clone().ok()?;
    let mut command = Command::new(program.binary);
    command
        .current_dir(cwd)
        .env("FFM_COMPLETE_LINE", &line)
        .args(["-c", "complete --do-complete=\"$FFM_COMPLETE_LINE\""])
        .stdin(Stdio::null())
        .stdout(stdout_writer)
        .stderr(Stdio::null());
    #[cfg(any(test, tarpaulin))]
    keep_out_of_the_home(&mut command);
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(err) => {
            if program.memoize_missing && err.kind() == ErrorKind::NotFound {
                FISH_UNAVAILABLE.store(true, Ordering::Relaxed);
            }
            return None;
        }
    };

    wait_for_fish(&mut child, stdout)
}

#[cfg(any(test, tarpaulin))]
fn keep_out_of_the_home(command: &mut Command) {
    const NOWHERE: &str = "/dev/null/ffm-tests";
    command
        .env("XDG_CONFIG_HOME", NOWHERE)
        .env("XDG_DATA_HOME", NOWHERE);
}

fn wait_for_fish(child: &mut std::process::Child, stdout_file: File) -> Option<Vec<String>> {
    let deadline = Instant::now() + FISH_COMPLETE_TIMEOUT;
    let mut poll = POLL_START;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    return None;
                }
                return read_fish_output(stdout_file);
            }
            Ok(None) if Instant::now() < deadline => {
                std::thread::sleep(poll);
                poll = (poll * 2).min(POLL_MAX);
            }
            Ok(None) | Err(_) => {
                drop(child.kill());
                drop(child.wait());
                return None;
            }
        }
    }
}

fn read_fish_output(mut stdout_file: File) -> Option<Vec<String>> {
    stdout_file.rewind().ok()?;
    let mut stdout = Vec::new();
    if let Err(error) = stdout_file.take(MAX_FISH_OUTPUT).read_to_end(&mut stdout) {
        crate::diagnostics::log_error(&format!("fish completion output: {error}"));
    }
    let candidates = parse_fish_completions(&String::from_utf8_lossy(&stdout));
    if candidates.is_empty() {
        None
    } else {
        Some(candidates)
    }
}

/// Parse `fish --do-complete` output: one candidate per line as `value\tdescription`.
pub(super) fn parse_fish_completions(stdout: &str) -> Vec<String> {
    stdout
        .lines()
        .map(|line| {
            line.split_once('\t')
                .map_or(line, |(value, _description)| value)
        })
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .collect()
}

#[cfg(test)]
mod tests;
