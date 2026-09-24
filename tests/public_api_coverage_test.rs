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

/// Public functions no test names yet.
///
/// Captured 2026-09-22 from the tree as it stood; every entry is a gap, not an
/// exemption. This list may only shrink — [`the_baseline_has_no_stale_entries`]
/// fails when an entry becomes testable and is left here.
const UNEXERCISED: &[&str] = &[
    "apply_navigation",
    "association_command",
    "canonical_existing_ancestor",
    "changes_bits",
    "clear_with_lock",
    "close_dialog",
    "copy_dir_all_with",
    "copy_edit_password",
    "cursor_column",
    "data_file",
    "default_mode",
    "edit_vault_if_kdbx",
    "expire",
    "finish_word",
    "follows_link",
    "from_last",
    "is_backward",
    "is_toggle",
    "last_activity",
    "link_choices",
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
    "save_session",
    "show_tree",
    "symlink_dir",
    "to_json",
    "to_search",
    "to_yaml",
    "toggle_video_mode",
    "undo_last_change",
    "visible_text",
    "with_stdout",
    "write_owner_only",
];

/// A file holding tests rather than production code.
fn is_test_source(path: &str) -> bool {
    path.contains("/tests/")
        || path.contains("\\tests\\")
        || Path::new(path)
            .file_stem()
            .and_then(|stem| stem.to_str())
            .is_some_and(|stem| stem.starts_with("tests"))
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

/// Names of every `pub fn` and `pub(crate) fn` declared in production sources.
fn public_functions(root: &Path) -> BTreeSet<String> {
    let mut files = Vec::new();
    rust_files(&root.join("src"), &mut files);
    let mut names = BTreeSet::new();
    for path in files {
        let display = path.display().to_string();
        if is_test_source(&display) {
            continue;
        }
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        for line in text.lines() {
            if let Some(name) = declared_name(line.trim_start()) {
                names.insert(name);
            }
        }
    }
    names
}

/// The function name in `pub fn name(`, or `None` for any other line.
///
/// Deliberately textual: pulling in a parser to read a declaration this regular
/// would cost more than the guard is worth.
fn declared_name(line: &str) -> Option<String> {
    let rest = line
        .strip_prefix("pub(crate) ")
        .or_else(|| line.strip_prefix("pub "))?;
    let rest = rest.strip_prefix("const ").unwrap_or(rest);
    let rest = rest.strip_prefix("async ").unwrap_or(rest);
    let rest = rest.strip_prefix("unsafe ").unwrap_or(rest);
    let rest = rest.strip_prefix("fn ")?;
    let name: String = rest
        .chars()
        .take_while(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '_')
        .collect();
    (!name.is_empty()).then_some(name)
}

/// Every test source in the crate, concatenated.
fn test_corpus(root: &Path) -> String {
    let mut files = Vec::new();
    rust_files(&root.join("src"), &mut files);
    rust_files(&root.join("tests"), &mut files);
    files
        .iter()
        .filter(|path| is_test_source(&path.display().to_string()))
        .filter_map(|path| fs::read_to_string(path).ok())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Whether a test calls the function or names its path.
fn is_exercised(corpus: &str, name: &str) -> bool {
    corpus.contains(&format!("{name}(")) || corpus.contains(&format!("::{name}"))
}

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

/// The parser must read the declaration forms this crate actually uses.
#[test]
fn declared_name_reads_the_forms_the_crate_uses() {
    assert_eq!(declared_name("pub fn open(").as_deref(), Some("open"));
    assert_eq!(
        declared_name("pub(crate) fn touch(&mut self)").as_deref(),
        Some("touch")
    );
    assert_eq!(
        declared_name("pub const fn width() -> u16").as_deref(),
        Some("width")
    );
    assert_eq!(declared_name("fn private_helper()"), None);
    assert_eq!(declared_name("pub struct App {"), None);
    assert_eq!(declared_name("// pub fn commented_out()"), None);
}

/// A test file must be recognised wherever it lives, on either path separator.
#[test]
fn test_sources_are_recognised_by_name_and_by_directory() {
    assert!(is_test_source("src/kdbx/tests.rs"));
    assert!(is_test_source("src/ops/tests/shell_commands.rs"));
    assert!(is_test_source("src/complete/tests_engines.rs"));
    assert!(!is_test_source("src/kdbx/vault.rs"));
    assert!(!is_test_source("src/casefold.rs"));
}
