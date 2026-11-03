//! Purpose: Stamp-validated cache for per-directory `git check-ignore` results.

use std::collections::HashMap;
use std::fs;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime};

const IGNORE_CACHE_TTL: Duration = Duration::from_secs(30);
const MAX_CACHED_DIRS: usize = 256;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct IgnoreStamp {
    dir_stamp: Option<(Option<SystemTime>, u64)>,
    index_stamp: Option<(Option<SystemTime>, u64)>,
    rules_hash: u64,
    names_hash: u64,
}

struct CachedIgnored {
    stamp: IgnoreStamp,
    checked_at: Instant,
    ignored: Vec<String>,
}

fn ignore_cache() -> &'static Mutex<HashMap<PathBuf, CachedIgnored>> {
    static CACHE: OnceLock<Mutex<HashMap<PathBuf, CachedIgnored>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

impl IgnoreStamp {
    pub(super) fn capture(
        directory: &Path,
        repo_root: &Path,
        git_dir: &Path,
        names: &[&str],
    ) -> Self {
        Self {
            dir_stamp: metadata_stamp(directory),
            index_stamp: metadata_stamp(&git_dir.join("index")),
            rules_hash: hash_rules(directory, repo_root, git_dir),
            names_hash: hash_names(names),
        }
    }
}

fn metadata_stamp(path: &Path) -> Option<(Option<SystemTime>, u64)> {
    let meta = fs::metadata(path).ok()?;
    Some((meta.modified().ok(), meta.len()))
}

fn hash_names(names: &[&str]) -> u64 {
    let mut hasher = DefaultHasher::new();
    names.hash(&mut hasher);
    hasher.finish()
}

fn hash_rules(directory: &Path, repo_root: &Path, git_dir: &Path) -> u64 {
    let mut hasher = DefaultHasher::new();
    hash_file(&git_dir.join("info").join("exclude"), &mut hasher);
    for ancestor in directory.ancestors() {
        hash_file(&ancestor.join(".gitignore"), &mut hasher);
        if ancestor == repo_root {
            break;
        }
    }
    hasher.finish()
}

fn hash_file(path: &Path, hasher: &mut DefaultHasher) {
    if let Ok(bytes) = fs::read(path) {
        true.hash(hasher);
        bytes.hash(hasher);
    } else {
        false.hash(hasher);
    }
}

pub(super) fn lookup(directory: &Path, stamp: IgnoreStamp) -> Option<Vec<String>> {
    let guard = ignore_cache()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let cached = guard.get(directory)?;
    let fresh = cached.stamp == stamp
        && Instant::now().saturating_duration_since(cached.checked_at) < IGNORE_CACHE_TTL;
    let result = fresh.then(|| cached.ignored.clone());
    drop(guard);
    result
}

pub(super) fn store(directory: &Path, stamp: IgnoreStamp, ignored: &[String]) {
    let mut guard = ignore_cache()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if guard.len() >= MAX_CACHED_DIRS && !guard.contains_key(directory) {
        guard.clear();
    }
    guard.insert(
        directory.to_path_buf(),
        CachedIgnored {
            stamp,
            checked_at: Instant::now(),
            ignored: ignored.to_vec(),
        },
    );
}

pub(in crate::git) fn invalidate_under(repo_root: &Path) {
    ignore_cache()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .retain(|dir, _| !dir.starts_with(repo_root));
}

#[cfg(test)]
mod tests;
