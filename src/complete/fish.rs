//! Purpose: the fish engine — spawn `complete --do-complete` for a `cd` line, bounded by a timeout, and parse what it prints.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Bound on a fish completion spawn so a hung fish cannot freeze the TUI.
/// Shell commands use a longer cap; Tab must stay snappy and fall back to the
/// built-in scan when fish is slow or stuck.
const FISH_COMPLETE_TIMEOUT: Duration = Duration::from_millis(1500);

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
    // Pass the line via an env var so fish does the quoting, not us.
    let mut child = Command::new(fish_program())
        .current_dir(cwd)
        .env("FFM_COMPLETE_LINE", &line)
        .args(["-c", "complete --do-complete=\"$FFM_COMPLETE_LINE\""])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;

    wait_for_fish(&mut child)
}

fn wait_for_fish(child: &mut std::process::Child) -> Option<Vec<String>> {
    let deadline = Instant::now() + FISH_COMPLETE_TIMEOUT;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    return None;
                }
                let mut stdout = Vec::new();
                let read = child
                    .stdout
                    .take()
                    .map(|mut pipe| pipe.read_to_end(&mut stdout));
                if let Some(Err(error)) = read {
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
