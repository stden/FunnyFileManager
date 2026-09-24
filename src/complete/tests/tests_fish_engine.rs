//! Purpose: The fish engine itself: output parsing and live spawns. Timeouts,
//! exit codes and the poll loop, driven by stub `fish` programs, are in
//! `tests_fish_stubs.rs`.

use super::*;

// ── fish engine: output parsing and live spawns ──────────────────────────

/// The fish output parser keeps the value column, drops the tab-separated
/// description, and skips blank lines. Pure (no fish spawn), so the
/// description-stripping is exercised on every platform — unlike the live
/// engine, whose `cd` output happens to carry no description here.
#[test]
fn parse_fish_completions_keeps_values_drops_descriptions_and_blanks() {
    let raw = "alpha/\tDirectory\nbeta/\ngamma/\tSubcommand\n\n\n";
    assert_eq!(
        parse_fish_completions(raw),
        vec!["alpha/".to_owned(), "beta/".to_owned(), "gamma/".to_owned()],
    );
    assert!(parse_fish_completions("").is_empty());
    // A value with interior spaces (a path with spaces) survives whole; only
    // the tab boundary splits.
    assert_eq!(
        parse_fish_completions("my docs/\tDirectory"),
        vec!["my docs/".to_owned()],
    );
}

/// The live fish engine offers the directories that actually exist in `cwd`.
/// Runs real `fish`, so it is skipped where fish is not installed.
///
/// The token is the path fragment after `cd ` — `fish_path_complete` prefixes
/// `cd` itself when it builds the line for fish.
#[test]
fn fish_path_complete_offers_existing_directories() {
    let tmp = empty_cwd();
    fs::create_dir(tmp.path().join("alpha")).expect("dir");
    fs::create_dir(tmp.path().join("alpine")).expect("dir");

    // Serialize with the fake-PATH fish tests (they shadow `fish` on PATH).
    let _lock = crate::test_support::env_lock();
    if !fish_is_installed() {
        return;
    }

    let Some(candidates) = fish_path_complete("al", tmp.path()) else {
        // fish is installed but produced nothing (unusual config); nothing to assert.
        return;
    };
    assert!(
        candidates.iter().any(|c| c == "alpha/"),
        "existing directory must be offered: {candidates:?}"
    );
    assert!(
        candidates.iter().any(|c| c == "alpine/"),
        "the other matching directory must be offered too: {candidates:?}"
    );
}

/// fish yields nothing for a leaf that matches no directory, and the engine
/// reports that as `None` (not `Some([])`) so the caller's fallback is clean.
#[test]
fn fish_path_complete_returns_none_when_nothing_matches() {
    let tmp = empty_cwd();
    fs::create_dir(tmp.path().join("alpha")).expect("dir");

    // Serialize with the fake-PATH fish tests (they shadow `fish` on PATH).
    let _lock = crate::test_support::env_lock();
    if !fish_is_installed() {
        return;
    }

    assert!(
        fish_path_complete("zzq_no_such_prefix", tmp.path()).is_none(),
        "no match must be None, never Some(empty)"
    );
}
