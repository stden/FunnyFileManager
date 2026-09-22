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

use std::fs;
use std::path::{Path, PathBuf};

/// Characters of a commit hash shown when the head is detached.
///
/// Seven is what git itself abbreviates to by default, and what a reader
/// recognises as "a short SHA" rather than as a truncated word.
const SHORT_HASH: usize = 7;

/// The branch `directory` is on, or a short commit hash when the head is
/// detached. `None` when this is not a repository.
#[must_use]
pub fn branch(directory: &Path) -> Option<String> {
    let git_dir = find_git_dir(directory)?;
    let head = fs::read_to_string(git_dir.join("HEAD")).ok()?;
    head_label(head.trim())
}

/// The `.git` directory governing `directory`, walking up from it.
///
/// `.git` is not always a directory: in a worktree or a submodule it is a file
/// holding `gitdir: <path>`, and following that is the difference between
/// naming the branch and reporting no repository at all.
fn find_git_dir(directory: &Path) -> Option<PathBuf> {
    for ancestor in directory.ancestors() {
        let candidate = ancestor.join(".git");
        if candidate.is_dir() {
            return Some(candidate);
        }
        if candidate.is_file() {
            let pointer = fs::read_to_string(&candidate).ok()?;
            let target = pointer.trim().strip_prefix("gitdir:")?.trim();
            let target = Path::new(target);
            // The path may be relative to the directory holding the `.git` file.
            return Some(if target.is_absolute() {
                target.to_path_buf()
            } else {
                ancestor.join(target)
            });
        }
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
        return Some(name.strip_prefix("refs/heads/").unwrap_or(name).to_string());
    }
    // A detached head: a bare hash, which is shown abbreviated.
    let looks_like_hash = head.len() >= SHORT_HASH && head.chars().all(|ch| ch.is_ascii_hexdigit());
    looks_like_hash.then(|| head.chars().take(SHORT_HASH).collect())
}

#[cfg(test)]
mod tests;
