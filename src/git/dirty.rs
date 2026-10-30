//! Fast, cached, non-blocking dirty-status inspection and background probing.

use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime};

use super::repo_git_dir;
use crate::models::{DirEntry, DirtyProbe, FileName, PanelMetadata};

mod probe;
use probe::{query_dirty, IN_PROGRESS};

/// How long an unchanged repository stamp reuses its last `git status` result.
const REPO_CACHE_TTL: Duration = Duration::from_secs(300);

#[derive(Clone, Copy, PartialEq, Eq)]
struct RepoStamp {
    root_modified: Option<SystemTime>,
    index_modified: Option<SystemTime>,
    head_modified: Option<SystemTime>,
    in_progress: u8,
}

#[derive(Clone, Copy)]
struct CachedRepo {
    stamp: RepoStamp,
    checked_at: Instant,
    dirty: bool,
}

fn repo_cache() -> &'static Mutex<HashMap<PathBuf, CachedRepo>> {
    static CACHE: OnceLock<Mutex<HashMap<PathBuf, CachedRepo>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

impl RepoStamp {
    fn capture(repo_path: &Path, git_dir: &Path) -> Self {
        let mut mask: u8 = 0;
        for (idx, name) in IN_PROGRESS.iter().enumerate() {
            if git_dir.join(name).exists() {
                mask |= 1_u8.wrapping_shl(u32::try_from(idx).unwrap_or(0));
            }
        }
        let mtime = |p: &Path| fs::metadata(p).and_then(|m| m.modified()).ok();
        Self {
            root_modified: mtime(repo_path),
            index_modified: mtime(&git_dir.join("index")),
            head_modified: mtime(&git_dir.join("HEAD")),
            in_progress: mask,
        }
    }
}

/// Whether `directory` is itself a Git repository root with uncommitted changes.
#[must_use]
pub fn is_dirty_repo(directory: &Path) -> bool {
    repo_git_dir(directory).is_some_and(|git_dir| query_dirty(directory, &git_dir))
}

/// Populate `metadata` from fast filesystem stamps and cached results, queuing
/// uncached child repositories for background `git status` probing.
pub fn load_dirty_repos(parent: &Path, entries: &[DirEntry], metadata: &mut PanelMetadata) {
    let (dirty_repos, pending) = inspect_dirty_repos(parent, entries);
    metadata.dirty_repos = dirty_repos;
    metadata.dirty_probe = DirtyProbe::from_pending(pending);
}

/// Classify direct child repositories of `parent` without spawning `git`.
#[must_use]
pub(super) fn inspect_dirty_repos(
    parent: &Path,
    entries: &[DirEntry],
) -> (BTreeSet<FileName>, Vec<FileName>) {
    let mut dirty_names = BTreeSet::new();
    let mut pending = Vec::new();
    let now = Instant::now();
    for entry in entries.iter().filter(|e| e.kind.is_dir()) {
        let repo_path = parent.join(&entry.name);
        let Some(git_dir) = repo_git_dir(&repo_path) else {
            continue;
        };
        let stamp = RepoStamp::capture(&repo_path, &git_dir);
        let name = FileName::new(entry.name.clone());
        match fast_or_cached(&repo_path, stamp, now) {
            Some(true) => {
                dirty_names.insert(name);
            }
            Some(false) => {}
            None => pending.push(name),
        }
    }
    (dirty_names, pending)
}

/// Probe a single child repository on a background worker and cache the answer.
#[must_use]
pub fn probe_child_repo(parent: &Path, name: &FileName) -> bool {
    let repo_path = parent.join(name.as_str());
    let Some(git_dir) = repo_git_dir(&repo_path) else {
        return false;
    };
    let now = Instant::now();
    let stamp = RepoStamp::capture(&repo_path, &git_dir);
    if let Some(dirty) = fast_or_cached(&repo_path, stamp, now) {
        return dirty;
    }
    let dirty = query_dirty(&repo_path, &git_dir);
    store_cached(repo_path, stamp, now, dirty);
    dirty
}

fn fast_or_cached(repo_path: &Path, stamp: RepoStamp, now: Instant) -> Option<bool> {
    if stamp.in_progress != 0 {
        store_cached(repo_path.to_path_buf(), stamp, now, true);
        return Some(true);
    }
    let cached = repo_cache().lock().ok()?.get(repo_path).copied()?;
    (cached.stamp == stamp && now.saturating_duration_since(cached.checked_at) < REPO_CACHE_TTL)
        .then_some(cached.dirty)
}

fn store_cached(repo_path: PathBuf, stamp: RepoStamp, checked_at: Instant, dirty: bool) {
    if let Ok(mut guard) = repo_cache().lock() {
        guard.insert(
            repo_path,
            CachedRepo {
                stamp,
                checked_at,
                dirty,
            },
        );
    }
}

/// Drop cached dirty status for the repository containing `path`.
pub fn invalidate_repo_cache_for(path: &Path) {
    let Some(root) = path.ancestors().find(|dir| repo_git_dir(dir).is_some()) else {
        return;
    };
    if let Ok(mut guard) = repo_cache().lock() {
        guard.remove(root);
    }
}

#[cfg(test)]
mod tests;
