//! Purpose: every public function must be exercised by a test, and the list of
//! the ones that are not may only get shorter.
//!
//! `AGENTS.md` has said "every `pub fn` in `src/` must have at least one test"
//! since before this guard existed, and nothing checked it. Line coverage in
//! CI does not: it is one number over the whole crate, so a new untested
//! function hides inside a rounding margin — which is exactly how coverage
//! drifted from 98.03 % to 97.77 % across one day of refactoring without any
//! single commit looking wrong.
//!
//! # What this proves, and what it does not
//!
//! The check is textual: a function counts as exercised when its name appears
//! in a test as a call `name(` or a path `::name`. That is weaker than real
//! coverage — a common name like `new` matches trivially, and a function
//! reached only indirectly through a caller counts as unexercised even though
//! it runs. It is also *stronger* than coverage in one way: it names the thing
//! that is missing instead of reporting a percentage, so a regression points
//! at the function that caused it.
//!
//! So this is a ratchet, not a proof. Use it with the CI coverage gate, not
//! instead of it.
//!
//! # Fixing a failure
//!
//! - **A new name appeared**: write the test. That is the whole point.
//! - **A name in [`UNEXERCISED`] is now tested**: delete it from the list. The
//!   guard requires this — a baseline nobody prunes stops being a baseline.
//! - **The function genuinely cannot be tested** (real TTY, real subprocess,
//!   another platform): add it to [`UNEXERCISED`] *with a comment saying
//!   which*, and say so in the pull request. Growing the list is a decision,
//!   not a formality.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

#[path = "public_api_coverage_test/corpus.rs"]
mod corpus;
use corpus::{is_exercised, public_functions, test_corpus};

/// Public functions no test names yet.
///
/// Captured 2026-09-22 from the tree as it stood; every entry is a gap, not an
/// exemption. This list may only shrink — [`the_baseline_has_no_stale_entries`]
/// fails when an entry becomes testable and is left here.
const UNEXERCISED: &[&str] = &[
    "association_command",
    // Windows: sends WM_SETTINGCHANGE to every window on the machine.
    "broadcast_environment_change",
    "canonical_existing_ancestor",
    "changes_bits",
    "clear_with_lock",
    "close_dialog",
    "copy_edit_password",
    "cursor_column",
    "data_file",
    "default_mode",
    "edit_vault_if_kdbx",
    "expire",
    "follows_link",
    "from_last",
    "is_backward",
    "is_toggle",
    "last_activity",
    // Raises a UAC prompt; a test must not ask the person at the console.
    "launch_as_administrator",
    "link_choices",
    // Stores a process-wide flag and would turn logging on for every later test.
    "log_by_default",
    "maintenance",
    "move_home",
    "needs_target",
    "offers_links",
    "open_password",
    "origin_id",
    "panel_areas",
    "panel_mut",
    "pop_char",
    "pop_filter_char",
    "push_filter_char",
    "read_catalog",
    "remember_marks",
    "run_command",
    "show_tree",
    "symlink_dir",
    "to_json",
    "to_search",
    "to_yaml",
    "toggle_video_mode",
    "undo_last_change",
    "visible_text",
    // Installs process-wide SIGHUP/SIGTERM handlers, which would let the test
    // runner outlive a SIGTERM; tests/pty_session_test/hang_up.rs signals the
    // real binary instead.
    "watch_for_hang_up",
    "with_stdout",
    "write_owner_only",
];

#[test]
fn every_public_function_is_exercised_by_a_test() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let corpus = test_corpus(root);
    let allowed: BTreeSet<&str> = UNEXERCISED.iter().copied().collect();

    let mut offenders: Vec<String> = public_functions(root)
        .into_iter()
        .filter(|name| !allowed.contains(name.as_str()))
        .filter(|name| !is_exercised(&corpus, name))
        .collect();
    offenders.sort();

    assert!(
        offenders.is_empty(),
        "these public functions are not named by any test — write one, or add \
         the name to UNEXERCISED with a reason:\n{}",
        offenders.join("\n")
    );
}

/// The baseline may only shrink: an entry that is now tested must be removed.
///
/// Without this, the list silently becomes a place to hide things, and the
/// guard degrades into decoration.
#[test]
fn the_baseline_has_no_stale_entries() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let corpus = test_corpus(root);
    let declared = public_functions(root);

    let mut stale: Vec<&str> = UNEXERCISED
        .iter()
        .copied()
        .filter(|name| !declared.contains(*name) || is_exercised(&corpus, name))
        .collect();
    stale.sort_unstable();

    assert!(
        stale.is_empty(),
        "UNEXERCISED is out of date — these are now tested, or no longer exist. \
         Delete them from the list:\n{}",
        stale.join("\n")
    );
}
