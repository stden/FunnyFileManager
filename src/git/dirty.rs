//! Fast, cached, parallel dirty-status probing for Git repository folders.

use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime};

use super::repo_git_dir;
use crate::models::{DirEntry, FileName};

/// State files inside `.git` that mark an unfinished operation.
const IN_PROGRESS: [&str; 4] = [
    "MERGE_HEAD",
    "REBASE_HEAD",
    "CHERRY_PICK_HEAD",
    "REVERT_HEAD",
];

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

/// Check all direct child directories of `parent` in parallel, reusing cached results.
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

type ProbeResult = (String, PathBuf, RepoStamp, bool);

fn probe_misses(misses: Vec<RepoProbe<'_>>, now: Instant, dirty_names: &mut BTreeSet<FileName>) {
    if misses.is_empty() {
        return;
    }
    let results = run_parallel_probes(misses);
    let Ok(mut guard) = repo_cache().lock() else {
        return;
    };
    for (name, repo_path, stamp, dirty) in results {
        let cached = CachedRepo {
            stamp,
            checked_at: now,
            dirty,
        };
        guard.insert(repo_path, cached);
        if dirty {
            dirty_names.insert(FileName::new(name));
        }
    }
}

fn run_parallel_probes(misses: Vec<RepoProbe<'_>>) -> Vec<ProbeResult> {
    let workers = misses.len().min(6);
    let queue = Mutex::new(misses.into_iter());
    std::thread::scope(|scope| {
        let mut handles = Vec::with_capacity(workers);
        for _ in 0..workers {
            handles.push(scope.spawn(|| drain_probes(&queue)));
        }
        handles
            .into_iter()
            .filter_map(|h| h.join().ok())
            .flatten()
            .collect()
    })
}

fn drain_probes(queue: &Mutex<std::vec::IntoIter<RepoProbe<'_>>>) -> Vec<ProbeResult> {
    let mut local = Vec::new();
    while let Some(p) = queue.lock().ok().and_then(|mut q| q.next()) {
        let dirty = query_dirty(&p.repo_path, &p.git_dir);
        local.push((p.name.to_owned(), p.repo_path, p.stamp, dirty));
    }
    local
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

fn query_dirty(directory: &Path, git_dir: &Path) -> bool {
    if IN_PROGRESS.iter().any(|name| git_dir.join(name).exists()) {
        return true;
    }
    let mut command = Command::new("git");
    command
        .args(["--no-optional-locks", "-C"])
        .arg(directory)
        .args(["status", "--porcelain", "-unormal", "--no-renames"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    let Ok(mut child) = command.spawn() else {
        return false;
    };
    let mut buf = [0_u8; 1];
    let dirty = child
        .stdout
        .take()
        .and_then(|mut out| out.read(&mut buf).ok())
        .is_some_and(|n| n > 0);
    if dirty {
        drop(child.kill());
    }
    drop(child.wait());
    dirty
}
