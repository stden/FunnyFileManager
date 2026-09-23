//! Purpose: `cd` path completion — the built-in directory scan, merged with fish's candidates and ranked by leaf.

use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};

use crate::command::expand_tilde;

use super::fish::fish_path_complete;
use super::matcher;

/// Complete a `cd` path argument by merging both engines.
///
/// The built-in fuzzy scan always contributes: fish only offers prefix
/// matches, and the design promises `dwn` matches `Downloads/` whether or
/// not fish is installed. fish's own candidates are merged in on top — they
/// add what a directory scan cannot know (case-corrected fallbacks, `CDPATH`
/// hits) — rather than being trusted exclusively.
pub(super) fn path_complete(token: &str, cwd: &Path) -> Vec<String> {
    let (dir_part, leaf) = split_token(token);
    let mut pool: Vec<String> = builtin_path_complete(dir_part, leaf, cwd)
        .into_iter()
        .map(|l| format!("{dir_part}{l}"))
        .collect();
    // fish completes the last shell *word*, so a token holding a name with
    // spaces would come back stripped of its prefix — skip fish for those.
    if token.contains(char::is_whitespace) {
        return rank_by_leaf(leaf, pool);
    }
    if let Some(fish) = fish_path_complete(token, cwd) {
        // Candidate lists are one directory's entries — linear dedup is fine.
        for candidate in fish {
            if !pool.contains(&candidate) {
                pool.push(candidate);
            }
        }
    }
    rank_by_leaf(leaf, pool)
}

/// Split a `cd` token into its already-typed directory part (ending in `/`,
/// or empty) and the partial leaf name being completed.
fn split_token(token: &str) -> (&str, &str) {
    match token.rfind('/') {
        Some(i) => token.split_at(i + 1),
        None => ("", token),
    }
}

/// Split a path completion *candidate* (not a token being typed) into its
/// directory part and leaf, ignoring the candidate's own trailing directory
/// slash when finding the split point — so `"sub/handlers/"` splits as
/// `("sub/", "handlers/")`, not `("sub/handlers/", "")`.
fn split_leaf(candidate: &str) -> (&str, &str) {
    let searched = candidate.strip_suffix('/').unwrap_or(candidate);
    match searched.rfind('/') {
        Some(i) => candidate.split_at(i + 1),
        None => ("", candidate),
    }
}

/// Rank full path candidates by each one's own leaf (see [`split_leaf`]).
///
/// Scoring the leaf alone — never the `dir_part`-prefixed full path — keeps
/// an exact leaf-prefix match (e.g. `"complete_b"` for query `"com"`) in the
/// `Prefix` tier even deep inside a subdirectory, and keeps shared directory
/// text from polluting point scoring. Candidates whose leaf does not match
/// are appended after the ranked ones, in their incoming relative order,
/// never dropped: fish validated them (case-insensitive fallbacks score
/// `None` under [`matcher::score`]'s smartcase rule but are still real
/// completions).
pub(super) fn rank_by_leaf(leaf: &str, candidates: Vec<String>) -> Vec<String> {
    let (matching, mut rest): (Vec<String>, Vec<String>) = candidates
        .into_iter()
        .partition(|c| matcher::score(leaf, split_leaf(c).1).is_some());

    // Rank by leaf text via `matcher::rank`, then look each ranked leaf back
    // up to its original candidate — queued per leaf text, in first-seen
    // order, in case two candidates happen to share a leaf name.
    let mut by_leaf: HashMap<String, VecDeque<String>> = HashMap::new();
    let mut leaves = Vec::with_capacity(matching.len());
    for candidate in matching {
        let leaf_text = split_leaf(&candidate).1.to_owned();
        by_leaf
            .entry(leaf_text.clone())
            .or_default()
            .push_back(candidate);
        leaves.push(leaf_text);
    }

    let mut ranked: Vec<String> = matcher::rank(leaf, leaves)
        .into_iter()
        .filter_map(|leaf_text| by_leaf.get_mut(&leaf_text)?.pop_front())
        .collect();

    ranked.append(&mut rest);
    ranked
}

/// Built-in `cd` path completion: list entries of the directory implied by
/// `dir_part` whose names match `leaf` (see [`matcher::score`]). Directory
/// matches gain a trailing `/`. Returns bare leaves — no `dir_part` prefix —
/// which the caller reattaches before ranking; see [`rank_by_leaf`].
pub(super) fn builtin_path_complete(dir_part: &str, leaf: &str, cwd: &Path) -> Vec<String> {
    let search_dir = resolve_dir(dir_part, cwd);

    let Ok(entries) = std::fs::read_dir(&search_dir) else {
        return Vec::new();
    };

    entries
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            matcher::score(leaf, &name)?;
            let is_dir = e.file_type().is_ok_and(|t| t.is_dir());
            let suffix = if is_dir { "/" } else { "" };
            Some(format!("{name}{suffix}"))
        })
        .collect()
}

/// Resolve the directory referenced by a path's directory part, expanding a
/// leading `~` and joining relative parts onto `cwd`. Tilde expansion goes
/// through [`expand_tilde`], the same one `cd` uses, so completion cannot offer
/// a directory `cd` then fails to open.
pub(super) fn resolve_dir(dir_part: &str, cwd: &Path) -> PathBuf {
    let p = expand_tilde(Path::new(dir_part));
    if p.is_absolute() {
        p
    } else {
        cwd.join(p)
    }
}
