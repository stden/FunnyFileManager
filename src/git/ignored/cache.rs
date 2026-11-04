//! Purpose: Stamp-validated cache for per-directory `git check-ignore` results.

use std::collections::HashMap;
use std::fs;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::time::{Duration, Instant, SystemTime};

const IGNORE_CACHE_TTL: Duration = Duration::from_secs(300);
const MAX_CACHED_DIRS: usize = 256;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct FileStamp {
    modified: Option<SystemTime>,
    len: u64,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct IgnoreStamp {
    dir_stamp: Option<FileStamp>,
    index_stamp: Option<FileStamp>,
    rules_hash: u64,
    names_hash: u64,
}

struct CachedIgnored {
    stamp: IgnoreStamp,
    checked_at: Instant,
    ignored: Vec<String>,
}

fn lock_cache() -> MutexGuard<'static, HashMap<PathBuf, CachedIgnored>> {
    static CACHE: OnceLock<Mutex<HashMap<PathBuf, CachedIgnored>>> = OnceLock::new();
    CACHE
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
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

fn metadata_stamp(path: &Path) -> Option<FileStamp> {
    let meta = fs::metadata(path).ok()?;
    Some(FileStamp {
        modified: meta.modified().ok(),
        len: meta.len(),
    })
}

/// Order-independent multiset hash of `names` so re-sorting a panel keeps its cache hit.
/// Combining wrapping sum and rotated XOR prevents duplicate names from cancelling out.
fn hash_names(names: &[&str]) -> u64 {
    let mut sum = 0_u64;
    let mut xor = 0_u64;
    for &name in names {
        let mut hasher = DefaultHasher::new();
        name.hash(&mut hasher);
        let h = hasher.finish();
        sum = sum.wrapping_add(h);
        xor ^= h.rotate_left(17);
    }
    sum.wrapping_add(xor.rotate_left(31))
        .wrapping_add(u64::try_from(names.len()).unwrap_or(0))
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
    let guard = lock_cache();
    let cached = guard.get(directory)?;
    if cached.stamp != stamp
        || Instant::now().saturating_duration_since(cached.checked_at) >= IGNORE_CACHE_TTL
    {
        return None;
    }
    let ignored = cached.ignored.clone();
    drop(guard);
    Some(ignored)
}

pub(super) fn store(directory: &Path, stamp: IgnoreStamp, ignored: &[String]) {
    let mut guard = lock_cache();
    if guard.len() >= MAX_CACHED_DIRS && !guard.contains_key(directory) {
        if let Some(oldest) = guard
            .iter()
            .min_by_key(|(_, entry)| entry.checked_at)
            .map(|(dir, _)| dir.clone())
        {
            guard.remove(&oldest);
        }
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
    lock_cache().retain(|dir, _| !dir.starts_with(repo_root));
}

#[cfg(test)]
mod tests;
