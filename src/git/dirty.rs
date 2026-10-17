//! Fast, cached, parallel dirty-status probing for Git repository folders.

use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime};

use super::repo_git_dir;
use crate::models::{DirEntry, FileName};

/// State files inside `.git` that mark an unfinished operation even when the
/// worktree itself has no modified files.
const IN_PROGRESS: [&str; 4] = [
    "MERGE_HEAD",
    "REBASE_HEAD",
    "CHERRY_PICK_HEAD",
    "REVERT_HEAD",
];

/// How long an unchanged repository stamp reuses its last `git status` result.
const REPO_CACHE_TTL: Duration = Duration::from_secs(15);

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

struct RepoProbe<'a> {
    name: &'a str,
    repo_path: PathBuf,
    git_dir: PathBuf,
    stamp: RepoStamp,
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
        Self {
            root_modified: modified_time(repo_path),
            index_modified: modified_time(&git_dir.join("index")),
            head_modified: modified_time(&git_dir.join("HEAD")),
            in_progress: mask,
        }
    }
}

fn modified_time(path: &Path) -> Option<SystemTime> {
    fs::metadata(path).and_then(|meta| meta.modified()).ok()
}

/// Whether `directory` is itself a Git repository root with uncommitted
/// changes or an in-progress merge/rebase/cherry-pick/revert.
#[must_use]
pub fn is_dirty_repo(directory: &Path) -> bool {
    let Some(git_dir) = repo_git_dir(directory) else {
        return false;
    };
    query_dirty(directory, &git_dir)
}

/// Check all direct child directories of `parent` in parallel, reusing recent
/// cached results when a repository's `.git/index`, `.git/HEAD`, and root
/// timestamp have not changed.
#[must_use]
pub fn dirty_repos(parent: &Path, entries: &[DirEntry]) -> BTreeSet<FileName> {
    let mut dirty_names = BTreeSet::new();
    let mut misses = Vec::new();
    let now = Instant::now();

    for entry in entries.iter().filter(|e| e.kind.is_dir()) {
        let repo_path = parent.join(&entry.name);
        let Some(git_dir) = repo_git_dir(&repo_path) else {
            continue;
        };
        let stamp = RepoStamp::capture(&repo_path, &git_dir);
        if let Some(dirty) = lookup_cached(&repo_path, stamp, now) {
            if dirty {
                dirty_names.insert(FileName::new(entry.name.clone()));
            }
        } else {
            misses.push(RepoProbe {
                name: &entry.name,
                repo_path,
                git_dir,
                stamp,
            });
        }
    }
    probe_misses(misses, now, &mut dirty_names);
    dirty_names
}

fn lookup_cached(repo_path: &Path, stamp: RepoStamp, now: Instant) -> Option<bool> {
    let cached = repo_cache().lock().ok()?.get(repo_path).copied()?;
    (cached.stamp == stamp && now.saturating_duration_since(cached.checked_at) < REPO_CACHE_TTL)
        .then_some(cached.dirty)
}

fn probe_misses(misses: Vec<RepoProbe<'_>>, now: Instant, dirty_names: &mut BTreeSet<FileName>) {
    if misses.is_empty() {
        return;
    }
    let results = run_parallel_probes(misses);
    if let Ok(mut guard) = repo_cache().lock() {
        for (name, repo_path, stamp, dirty) in results {
            guard.insert(
                repo_path,
                CachedRepo {
                    stamp,
                    checked_at: now,
                    dirty,
                },
            );
            if dirty {
                dirty_names.insert(FileName::new(name));
            }
        }
    }
}

fn run_parallel_probes(misses: Vec<RepoProbe<'_>>) -> Vec<(String, PathBuf, RepoStamp, bool)> {
    std::thread::scope(|scope| {
        let mut handles = Vec::with_capacity(misses.len());
        for probe in misses {
            handles.push(scope.spawn(move || {
                let dirty = query_dirty(&probe.repo_path, &probe.git_dir);
                (probe.name.to_owned(), probe.repo_path, probe.stamp, dirty)
            }));
        }
        let mut out = Vec::with_capacity(handles.len());
        for handle in handles {
            if let Ok(item) = handle.join() {
                out.push(item);
            }
        }
        out
    })
}

/// Drop cached dirty status for the repository containing `path` so a reload
/// after a shell command or file operation re-checks the active worktree.
pub fn invalidate_repo_cache_for(path: &Path) {
    let Some(repo_root) = path.ancestors().find(|dir| repo_git_dir(dir).is_some()) else {
        return;
    };
    if let Ok(mut guard) = repo_cache().lock() {
        guard.remove(repo_root);
    }
}

fn query_dirty(directory: &Path, git_dir: &Path) -> bool {
    if IN_PROGRESS.iter().any(|name| git_dir.join(name).exists()) {
        return true;
    }
    let mut command = std::process::Command::new("git");
    command
        .args(["--no-optional-locks", "-C"])
        .arg(directory)
        .args(["status", "--porcelain", "-unormal"])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    command
        .output()
        .ok()
        .filter(|output| output.status.success())
        .is_some_and(|output| !output.stdout.is_empty())
}
