//! Purpose: Unit tests for the command-line tab-completion engine.
//!
use super::shellwords;
use super::*;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// A cwd that exists but is empty, for cases where the dir doesn't matter.
fn empty_cwd() -> TempDir {
    TempDir::new().expect("tempdir")
}

/// The `text` of each ranked candidate, for asserting on completion output.
fn texts(c: &Completion) -> Vec<String> {
    c.candidates.iter().map(|cand| cand.text.clone()).collect()
}

/// A cwd holding the given entries; directories are the names ending in `/`.
fn cwd_with(entries: &[&str]) -> TempDir {
    let tmp = TempDir::new().expect("tempdir");
    for entry in entries {
        match entry.strip_suffix('/') {
            Some(dir) => fs::create_dir(tmp.path().join(dir)).expect("mkdir"),
            None => fs::write(tmp.path().join(entry), b"x").expect("write"),
        }
    }
    tmp
}

/// Whether the fish engine `path_complete` merges in is actually installed.
/// Mirrors production: `fish_path_complete` gives up the same way when the
/// process cannot be spawned.
///
/// Callers that branch on this and then spawn fish must hold
/// [`crate::test_support::env_lock`] for the whole check+spawn window: the
/// fake-`PATH` tests temporarily shadow `fish`, and a concurrent lookup would
/// either hang on the sleeper or miss a real install.
fn fish_is_installed() -> bool {
    Command::new("fish").arg("--version").output().is_ok()
}

#[cfg(test)]
mod tests_bookmarks;
#[cfg(test)]
mod tests_cd_paths;
#[cfg(test)]
mod tests_engines;
#[cfg(test)]
mod tests_fish_engine;
#[cfg(test)]
mod tests_helpers;
#[cfg(test)]
mod tests_ranking;
#[cfg(test)]
mod tests_shell_args;
#[cfg(test)]
mod tests_verbs;
