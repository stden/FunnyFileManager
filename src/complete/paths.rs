//! Purpose: `cd` path completion — the built-in directory scan, merged with fish's candidates and ranked by leaf.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::command::expand_tilde;

use super::fish::fish_path_complete;
use super::{matcher, os_into_string, PathSyntax};

struct PooledCandidate {
    cached_score: Option<matcher::Score>,
    path: String,
}

struct ScoredCandidate {
    score: matcher::Score,
    leaf_offset: usize,
    path: String,
}

impl ScoredCandidate {
    fn leaf(&self) -> &str {
        self.path.get(self.leaf_offset..).unwrap_or(&self.path)
    }
}

/// Complete a `cd` path argument by merging both engines.
pub(super) fn path_complete(syntax: PathSyntax, token: &str, cwd: &Path) -> Vec<String> {
    if let Some(root) = syntax.drive_root(token) {
        let listable = std::fs::read_dir(resolve_dir(&root, cwd)).is_ok();
        return if listable { vec![root] } else { Vec::new() };
    }
    let (dir_part, leaf) = syntax.split_token(token);
    let prepared = matcher::PreparedQuery::new(leaf);
    let mut pool: Vec<PooledCandidate> = scan_builtin_leaves(syntax, dir_part, &prepared, cwd)
        .into_iter()
        .map(|(score, leaf_name)| {
            let path = if dir_part.is_empty() {
                leaf_name
            } else {
                format!("{dir_part}{leaf_name}")
            };
            PooledCandidate {
                cached_score: Some(score),
                path,
            }
        })
        .collect();
    if !token.contains(char::is_whitespace) {
        if let Some(fish) = fish_path_complete(token, cwd) {
            append_unique_unscored(&mut pool, fish);
        }
    }
    rank_scored_by_leaf(syntax, &prepared, pool)
}

fn append_unique_unscored(pool: &mut Vec<PooledCandidate>, incoming: Vec<String>) {
    let mut seen: HashSet<String> = pool.iter().map(|item| item.path.clone()).collect();
    pool.extend(
        incoming
            .into_iter()
            .filter(|candidate| seen.insert(candidate.clone()))
            .map(|path| PooledCandidate {
                cached_score: None,
                path,
            }),
    );
}

/// Rank full path candidates by each one's own leaf (see [`PathSyntax::split_leaf`]).
#[cfg(test)]
pub(super) fn rank_by_leaf(syntax: PathSyntax, leaf: &str, candidates: Vec<String>) -> Vec<String> {
    let prepared = matcher::PreparedQuery::new(leaf);
    let pool = candidates
        .into_iter()
        .map(|path| PooledCandidate {
            cached_score: None,
            path,
        })
        .collect();
    rank_scored_by_leaf(syntax, &prepared, pool)
}

fn rank_scored_by_leaf(
    syntax: PathSyntax,
    prepared: &matcher::PreparedQuery<'_>,
    candidates: Vec<PooledCandidate>,
) -> Vec<String> {
    let mut scored: Vec<ScoredCandidate> = Vec::with_capacity(candidates.len());
    let mut rest: Vec<String> = Vec::new();
    for item in candidates {
        let (dir_prefix, leaf_text) = syntax.split_leaf(&item.path);
        if let Some(score) = item.cached_score.or_else(|| prepared.score(leaf_text)) {
            scored.push(ScoredCandidate {
                score,
                leaf_offset: dir_prefix.len(),
                path: item.path,
            });
        } else {
            rest.push(item.path);
        }
    }
    scored.sort_by(|a, b| matcher::cmp_ranked(a.score, a.leaf(), b.score, b.leaf()));
    let mut ranked: Vec<String> = scored.into_iter().map(|item| item.path).collect();
    ranked.append(&mut rest);
    ranked
}

/// Built-in `cd` path completion returning bare leaves.
#[cfg(test)]
pub(super) fn builtin_path_complete(
    syntax: PathSyntax,
    dir_part: &str,
    leaf: &str,
    cwd: &Path,
) -> Vec<String> {
    let prepared = matcher::PreparedQuery::new(leaf);
    scan_builtin_leaves(syntax, dir_part, &prepared, cwd)
        .into_iter()
        .map(|(_, name)| name)
        .collect()
}

fn scan_builtin_leaves(
    syntax: PathSyntax,
    dir_part: &str,
    prepared: &matcher::PreparedQuery<'_>,
    cwd: &Path,
) -> Vec<(matcher::Score, String)> {
    let search_dir = resolve_dir(dir_part, cwd);
    let suffix = syntax.dir_suffix(dir_part);

    let Ok(entries) = std::fs::read_dir(&search_dir) else {
        return Vec::new();
    };

    entries
        .flatten()
        .filter_map(|e| {
            let mut name = os_into_string(e.file_name());
            let score = prepared.score(&name)?;
            if e.file_type().is_ok_and(|t| t.is_dir()) {
                name.push(suffix);
            }
            Some((score, name))
        })
        .collect()
}

/// Resolve the directory referenced by a path's directory part.
pub(super) fn resolve_dir(dir_part: &str, cwd: &Path) -> PathBuf {
    let p = expand_tilde(Path::new(dir_part));
    if p.is_absolute() {
        p
    } else {
        cwd.join(p)
    }
}
