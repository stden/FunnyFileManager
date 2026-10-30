//! Purpose: The git branch a directory is on, for the command prompt.
//!
//! Read straight out of `.git/HEAD`, the way Starship does it — not by running
//! `git`. A prompt is redrawn on every keystroke, and spawning a process per
//! frame would make the file manager pause on its own command line. This is one
//! small file read, and it answers on a repository whose `git` is not even
//! installed.
//!
//! Only the branch. Starship also counts staged, modified and untracked files;
//! each of those needs the index and a walk of the work tree, which is exactly
//! the cost this module exists to avoid.

use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// Characters of a commit hash shown when the head is detached.
///
/// Seven is what git itself abbreviates to by default, and what a reader
/// recognises as "a short SHA" rather than as a truncated word.
const SHORT_HASH: usize = 7;

/// How long a directory's branch answer is reused across consecutive frames
/// while the cursor moves inside the same panel directory.
const CACHE_TTL: Duration = Duration::from_secs(2);

struct CachedBranch {
    directory: PathBuf,
    checked_at: Instant,
    branch: Option<String>,
}

thread_local! {
    static LAST_BRANCH: RefCell<Option<CachedBranch>> = const { RefCell::new(None) };
}

/// The branch `directory` is on, or a short commit hash when the head is
/// detached. `None` when this is not a repository.
#[must_use]
pub fn branch(directory: &Path) -> Option<String> {
    let now = Instant::now();
    if let Some(cached) = LAST_BRANCH.with(|slot| {
        let borrow = slot.borrow();
        let entry = borrow.as_ref()?;
        (entry.directory == directory
            && now.saturating_duration_since(entry.checked_at) < CACHE_TTL)
            .then(|| entry.branch.clone())
    }) {
        return cached;
    }
    let resolved = read_branch(directory);
    LAST_BRANCH.with(|slot| {
        *slot.borrow_mut() = Some(CachedBranch {
            directory: directory.to_path_buf(),
            checked_at: now,
            branch: resolved.clone(),
        });
    });
    resolved
}

fn read_branch(directory: &Path) -> Option<String> {
    let git_dir = find_git_dir(directory)?;
    let head = fs::read_to_string(git_dir.join("HEAD")).ok()?;
    head_label(head.trim())
}

mod dirty;
#[cfg(test)]
use dirty::inspect_dirty_repos;
pub use dirty::is_dirty_repo;
pub(crate) use dirty::{invalidate_repo_cache_for, load_dirty_repos, probe_child_repo};

/// The `.git` directory governing `directory`, walking up from it.
///
/// `.git` is not always a directory: in a worktree or a submodule it is a file
/// holding `gitdir: <path>`, and following that is the difference between
/// naming the branch and reporting no repository at all.
fn find_git_dir(directory: &Path) -> Option<PathBuf> {
    directory.ancestors().find_map(repo_git_dir)
}

fn repo_git_dir(directory: &Path) -> Option<PathBuf> {
    let candidate = directory.join(".git");
    let metadata = fs::metadata(&candidate).ok()?;
    if metadata.is_dir() {
        return Some(candidate);
    }
    if metadata.is_file() {
        let pointer = fs::read_to_string(&candidate).ok()?;
        let target = pointer.trim().strip_prefix("gitdir:")?.trim();
        let target = Path::new(target);
        return Some(if target.is_absolute() {
            target.to_path_buf()
        } else {
            directory.join(target)
        });
    }
    None
}

/// What `HEAD`'s contents name.
///
/// Either a symbolic ref — `ref: refs/heads/main` — or, with the head detached,
/// the commit hash itself.
fn head_label(head: &str) -> Option<String> {
    if let Some(reference) = head.strip_prefix("ref:") {
        let name = reference.trim();
        // `refs/heads/feature/x` is the branch `feature/x`, so only the known
        // prefix is removed rather than everything up to the last slash.
        return Some(name.strip_prefix("refs/heads/").unwrap_or(name).to_owned());
    }
    // A detached head: a bare hash, which is shown abbreviated.
    let looks_like_hash = head.len() >= SHORT_HASH && head.chars().all(|ch| ch.is_ascii_hexdigit());
    looks_like_hash.then(|| head.chars().take(SHORT_HASH).collect())
}

#[cfg(test)]
mod tests;
