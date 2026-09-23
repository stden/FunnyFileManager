//! Purpose: Tab-completion for the `:` command line (verbs, sub-commands, paths).
//!
//! Verb and sub-command completion is table-driven; `cd` path completion
//! delegates to the **fish** shell's completion engine when `fish` is on `PATH`
//! (the intended setup on macOS), and falls back to a built-in directory scan
//! so completion still works — and stays testable — without fish installed.

use std::path::{Path, PathBuf};

use crate::models::{Candidate, CandidateKind};

mod candidates;
pub mod execs;
mod fish;
pub mod matcher;
mod paths;
pub mod shellwords;

use candidates::candidates_for;

// The unit tests in `tests.rs` reach these seams through `use super::*`.
#[cfg(test)]
use candidates::{is_home_syntax, keep_typed_tilde};
#[cfg(test)]
use fish::{fish_path_complete, parse_fish_completions};
// The fish stub is driven only by the Unix-gated engine tests.
#[cfg(all(test, unix))]
use fish::FISH_PROGRAM_FOR_TESTS;
#[cfg(test)]
use paths::{builtin_path_complete, rank_by_leaf, resolve_dir};
// Reached from the Unix-gated merge tests; `candidates` imports it directly.
#[cfg(all(test, unix))]
use paths::path_complete;
#[cfg(test)]
use std::process::Command;

/// Sources a completion draws on beyond the raw input.
///
/// `cwd` is the active panel's directory; `bookmarks` are its slots (index =
/// slot number, `None` = empty). Phase 3 adds `bookmarks`; earlier phases
/// used only `cwd`.
pub struct CompletionContext<'a> {
    pub cwd: &'a Path,
    pub bookmarks: &'a [Option<PathBuf>],
}

impl<'a> CompletionContext<'a> {
    /// Context with no bookmark slots — for positions that never complete a
    /// bookmark, and for tests that do not exercise bookmarks.
    #[must_use]
    pub const fn for_cwd(cwd: &'a Path) -> Self {
        Self {
            cwd,
            bookmarks: &[],
        }
    }
}

/// Outcome of a completion attempt.
pub struct Completion {
    /// The full replacement text for the command line.
    pub line: String,
    /// All matching candidates (len > 1 means the result was ambiguous and only
    /// advanced to the longest common prefix — useful to surface to the user).
    pub candidates: Vec<Candidate>,
}

/// Complete `input` (the current command-line text) given the active panel's
/// directory `cwd`. Returns `None` when there is nothing to complete.
#[must_use]
pub fn complete(input: &str, cwd: &Path) -> Option<Completion> {
    complete_with(input, &CompletionContext::for_cwd(cwd))
}

/// Complete `input` (the current command-line text) against `ctx`. Returns
/// `None` when there is nothing to complete.
#[must_use]
pub fn complete_with(input: &str, ctx: &CompletionContext<'_>) -> Option<Completion> {
    let (head, token) = split_target(input);
    let candidates = candidates_for(head, token, ctx);

    if candidates.is_empty() {
        return None;
    }

    let completed = if let [only] = candidates.as_slice() {
        finish_word(&only.text, only.kind)
    } else {
        // Ambiguous: advance to the longest common prefix only (no trailing space).
        let texts: Vec<String> = candidates.iter().map(|c| c.text.clone()).collect();
        let lcp = longest_common_prefix(&texts);
        if lcp.len() <= token.len() {
            // Already at the common prefix — nothing to insert, just report options.
            return Some(Completion {
                line: input.to_owned(),
                candidates,
            });
        }
        lcp
    };

    Some(Completion {
        line: format!("{head}{completed}"),
        candidates,
    })
}

/// Split `input` into the fixed prefix and the token being completed.
///
/// `cd` is the exception: it takes the whole rest of the line as ONE path
/// argument (see [`crate::command::Command::parse`]), so a directory name
/// containing spaces completes as a unit and parses back the same way.
/// `pub(crate)` because the dropdown's `Enter`-accept in [`crate::popup`] must
/// replace exactly the token this split identified.
pub(crate) fn split_target(input: &str) -> (&str, &str) {
    match input.trim_start().split_once(char::is_whitespace) {
        // `token` is a suffix of `input`, so stripping it leaves the head.
        Some(("cd", rest)) => {
            let token = rest.trim_start();
            (input.strip_suffix(token).unwrap_or_default(), token)
        }
        // Quote- and escape-aware, so `cat My\ Doc` completes against
        // `My Doc` rather than against `Doc`.
        _ => shellwords::last_word(input),
    }
}

/// Append a trailing space when accepting a candidate, except for directory
/// paths (ending in `/`) where the user likely wants to descend. Shared by
/// `Tab`'s lone-candidate accept and the dropdown's `Enter` accept, so both
/// finish a word the same way.
pub(crate) fn finish_word(candidate: &str, kind: CandidateKind) -> String {
    if kind == CandidateKind::Path && candidate.ends_with('/') {
        candidate.to_owned()
    } else {
        format!("{candidate} ")
    }
}

/// Longest common prefix of a non-empty candidate list, on `char` boundaries.
fn longest_common_prefix(candidates: &[String]) -> String {
    let Some((first, others)) = candidates.split_first() else {
        return String::new();
    };
    let mut prefix = first.as_str();
    for other in others {
        // `common_prefix_len` ends on a char boundary of `prefix`, so `get`
        // always succeeds.
        prefix = prefix
            .get(..common_prefix_len(prefix, other))
            .unwrap_or_default();
    }
    prefix.to_owned()
}

/// Byte length of the shared leading run of characters in `a` and `b`.
pub(crate) fn common_prefix_len(a: &str, b: &str) -> usize {
    a.chars()
        .zip(b.chars())
        .take_while(|(ca, cb)| ca == cb)
        .map(|(ca, _)| ca.len_utf8())
        .sum()
}

#[cfg(test)]
mod tests;
