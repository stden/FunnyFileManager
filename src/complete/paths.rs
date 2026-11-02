//! Purpose: `cd` path completion — the built-in directory scan, merged with fish's candidates and ranked by leaf.

use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};

use crate::command::expand_tilde;

use super::fish::fish_path_complete;
use super::{matcher, PathSyntax};

/// Complete a `cd` path argument by merging both engines.
///
/// The built-in fuzzy scan always contributes: fish only offers prefix
/// matches, and the design promises `dwn` matches `Downloads/` whether or
/// not fish is installed. fish's own candidates are merged in on top — they
/// add what a directory scan cannot know (case-corrected fallbacks, `CDPATH`
/// hits) — rather than being trusted exclusively.
pub(super) fn path_complete(syntax: PathSyntax, token: &str, cwd: &Path) -> Vec<String> {
    // `cd C:` names a drive, not a directory: offer its root, if listable.
    if let Some(root) = syntax.drive_root(token) {
        let listable = std::fs::read_dir(resolve_dir(&root, cwd)).is_ok();
        return if listable { vec![root] } else { Vec::new() };
    }
    let (dir_part, leaf) = syntax.split_token(token);
    let pool: Vec<String> = builtin_path_complete(syntax, dir_part, leaf, cwd)
        .into_iter()
        .map(|l| format!("{dir_part}{l}"))
        .collect();
    // fish completes the last shell *word*, so a token holding a name with
    // spaces would come back stripped of its prefix — skip fish for those.
    if token.contains(char::is_whitespace) {
        return rank_by_leaf(syntax, leaf, pool);
    }
    let pool = match fish_path_complete(token, cwd) {
        Some(fish) => super::merge::append_unique(pool, fish),
        None => pool,
    };
    rank_by_leaf(syntax, leaf, pool)
}

/// Rank full path candidates by each one's own leaf (see
/// [`PathSyntax::split_leaf`]).
///
/// Scoring the leaf alone — never the `dir_part`-prefixed full path — keeps
/// an exact leaf-prefix match (e.g. `"complete_b"` for query `"com"`) in the
/// `Prefix` tier even deep inside a subdirectory, and keeps shared directory
/// text from polluting point scoring. Candidates whose leaf does not match
/// are appended after the ranked ones, in their incoming relative order,
/// never dropped: fish validated them (case-insensitive fallbacks score
/// `None` under [`matcher::score`]'s smartcase rule but are still real
/// completions).
pub(super) fn rank_by_leaf(syntax: PathSyntax, leaf: &str, candidates: Vec<String>) -> Vec<String> {
    let (matching, mut rest): (Vec<String>, Vec<String>) = candidates
        .into_iter()
        .partition(|c| matcher::score(leaf, syntax.split_leaf(c).1).is_some());

    // Rank by leaf text via `matcher::rank`, then look each ranked leaf back
    // up to its original candidate — queued per leaf text, in first-seen
    // order, in case two candidates happen to share a leaf name.
    let mut by_leaf: HashMap<String, VecDeque<String>> = HashMap::new();
    let mut leaves = Vec::with_capacity(matching.len());
    for candidate in matching {
        let leaf_text = syntax.split_leaf(&candidate).1.to_owned();
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
/// matches gain a trailing separator — the one `dir_part` ends in, else the
/// syntax's own (see [`PathSyntax::dir_suffix`]). Returns bare leaves — no
/// `dir_part` prefix — which the caller reattaches before ranking; see
/// [`rank_by_leaf`].
pub(super) fn builtin_path_complete(
    syntax: PathSyntax,
    dir_part: &str,
    leaf: &str,
    cwd: &Path,
) -> Vec<String> {
    let search_dir = resolve_dir(dir_part, cwd);
    let suffix = syntax.dir_suffix(dir_part);

    let Ok(entries) = std::fs::read_dir(&search_dir) else {
        return Vec::new();
    };

    entries
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            matcher::score(leaf, &name)?;
            let is_dir = e.file_type().is_ok_and(|t| t.is_dir());
            Some(if is_dir {
                format!("{name}{suffix}")
            } else {
                name
            })
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
