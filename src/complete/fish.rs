//! Purpose: the fish engine — spawn `complete --do-complete` for a `cd` line, bounded by a timeout, and parse what it prints.

use std::fs::File;
use std::io::{Read, Seek};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Bound on a fish completion spawn so a hung fish cannot freeze the TUI.
/// Shell commands use a longer cap; Tab must stay snappy and fall back to the
/// built-in scan when fish is slow or stuck.
const FISH_COMPLETE_TIMEOUT: Duration = Duration::from_millis(1500);

/// Maximum completion bytes read back from fish stdout spool.
const MAX_FISH_OUTPUT: u64 = 1024 * 1024;

// Test override for the completion shell, following the crate's cfg-gated
// injection-point convention: the poll loop's slow path only shows itself
// when the child outlives a poll tick, which real `fish` rarely does.
// Thread-local so a test's stub cannot leak into tests running in parallel.
#[cfg(any(test, tarpaulin))]
thread_local! {
    pub(crate) static FISH_PROGRAM_FOR_TESTS: std::cell::RefCell<Option<PathBuf>> =
        const { std::cell::RefCell::new(None) };
}

/// The program spoken to as fish: the real one, unless a test injected a stub.
fn fish_program() -> PathBuf {
    #[cfg(any(test, tarpaulin))]
    if let Some(program) = FISH_PROGRAM_FOR_TESTS.with(|slot| slot.borrow().clone()) {
        return program;
    }
    PathBuf::from("fish")
}

/// Ask the fish shell for `cd <token>` completions, run with `cwd` as the
/// working directory. Returns `None` when fish is unavailable, times out, or
/// yields nothing, so the caller can fall back. fish prints one candidate per
/// line as `value\tdescription`; we keep the value (the full token replacement).
pub(super) fn fish_path_complete(token: &str, cwd: &Path) -> Option<Vec<String>> {
    let line = format!("cd {token}");
    let stdout = tempfile::tempfile().ok()?;
    let stdout_writer = stdout.try_clone().ok()?;
    // Pass the line via an env var so fish does the quoting, not us.
    let mut command = Command::new(fish_program());
    command
        .current_dir(cwd)
        .env("FFM_COMPLETE_LINE", &line)
        .args(["-c", "complete --do-complete=\"$FFM_COMPLETE_LINE\""])
        .stdin(Stdio::null())
        .stdout(stdout_writer)
        .stderr(Stdio::null());
    #[cfg(any(test, tarpaulin))]
    keep_out_of_the_home(&mut command);
    let mut child = command.spawn().ok()?;

    wait_for_fish(&mut child, stdout)
}

/// fish creates `fish/` in its configuration and data homes on every run
/// (`~/.config/fish`, `~/.local/share/fish`), and a test must not write into
/// the home of whoever runs the suite. Under a file nobody, root included, can
/// create a directory: fish keeps no history there, warns on the discarded
/// stderr, and completes as usual.
#[cfg(any(test, tarpaulin))]
fn keep_out_of_the_home(command: &mut Command) {
    const NOWHERE: &str = "/dev/null/ffm-tests";
    command
        .env("XDG_CONFIG_HOME", NOWHERE)
        .env("XDG_DATA_HOME", NOWHERE);
}

fn wait_for_fish(child: &mut std::process::Child, mut stdout_file: File) -> Option<Vec<String>> {
    let deadline = Instant::now() + FISH_COMPLETE_TIMEOUT;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    return None;
                }
                stdout_file.rewind().ok()?;
                let mut stdout = Vec::new();
                if let Err(error) = stdout_file.take(MAX_FISH_OUTPUT).read_to_end(&mut stdout) {
                    // Parse whatever arrived before the failure, as before.
                    crate::diagnostics::log_error(&format!("fish completion output: {error}"));
                }
                let candidates = parse_fish_completions(&String::from_utf8_lossy(&stdout));
                return (!candidates.is_empty()).then_some(candidates);
            }
            Ok(None) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(20));
            }
            // Timed out — or `try_wait` itself failed, which gets the same
            // treatment: kill whatever is left and fall back to no completion.
            Ok(None) | Err(_) => {
                // Best effort: fish may have exited on its own meanwhile, and
                // completion falls back either way.
                drop(child.kill());
                drop(child.wait());
                return None;
            }
        }
    }
}

/// Parse `fish --do-complete` output: one candidate per line as
/// `value\tdescription`. Keep the value column (the full token replacement),
/// drop the description, and skip blank lines. Split out from
/// [`fish_path_complete`] so the parsing is testable without spawning fish.
pub(super) fn parse_fish_completions(stdout: &str) -> Vec<String> {
    stdout
        .lines()
        .filter_map(|l| l.split('\t').next())
        .filter(|v| !v.is_empty())
        .map(str::to_owned)
        .collect()
}
