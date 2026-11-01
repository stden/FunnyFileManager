//! Purpose: What git says about one folder's entries — one listing of the
//! tracked paths and one of the ignored paths for the whole folder, never a call
//! per entry — and the parsers that read those listings.

use std::collections::HashSet;
use std::ffi::OsStr;
use std::io;
use std::path::Path;
use std::time::{Duration, Instant};

use crate::ops::git_pull::repositories::git;
use crate::space_verdict::{GitFacts, Ignoring, Tracking};

/// How long each listing may run before git is stopped. A stopped listing
/// leaves its folder unreadable, so nothing is called safe on a guess.
const LISTING_TIME: Duration = Duration::from_secs(3);

/// What git knows about the entries of one folder.
pub(super) enum Repo {
    Outside,
    InsideGitDir,
    Unreadable,
    Listed {
        tracked: HashSet<String>,
        ignored: HashSet<String>,
    },
}

impl Repo {
    /// What git says about the entry called `name`.
    pub(super) fn facts(&self, name: &str) -> GitFacts {
        match self {
            Self::Outside => GitFacts::Outside,
            Self::InsideGitDir => GitFacts::InsideGitDir,
            Self::Unreadable => GitFacts::Unreadable,
            Self::Listed { tracked, ignored } => GitFacts::Entry {
                tracking: if tracked.contains(name) {
                    Tracking::Tracked
                } else {
                    Tracking::Untracked
                },
                ignoring: if ignored.contains(name) {
                    Ignoring::Ignored
                } else {
                    Ignoring::NotIgnored
                },
            },
        }
    }

    /// What the dialog must say about git, when git cannot say everything.
    pub(super) fn note(&self) -> Option<&'static str> {
        match self {
            Self::Outside => Some("Not inside a git repository: only cache folders can be safe."),
            Self::InsideGitDir => {
                Some("Inside a .git folder: everything here is git's own and is kept.")
            }
            Self::Unreadable => {
                Some("git could not list this repository: nothing here is marked safe.")
            }
            Self::Listed { .. } => None,
        }
    }
}

/// The repository `folder` is in, if any, and what git lists in it. A folder
/// with a repository above it but no listing is unreadable, never outside git.
pub(super) fn survey(folder: &Path) -> Repo {
    if folder
        .ancestors()
        .any(|dir| dir.file_name() == Some(OsStr::new(".git")))
    {
        return Repo::InsideGitDir;
    }
    let Some(root) = repository_root(folder) else {
        return Repo::Outside;
    };
    listing(folder, root).unwrap_or(Repo::Unreadable)
}

/// The nearest folder above `folder` that git would take for a repository root.
/// A `.git` file always counts, since git follows its pointer and fails if it is
/// broken. A `.git` directory counts only with a `HEAD`: git skips an empty one.
fn repository_root(folder: &Path) -> Option<&Path> {
    folder.ancestors().find(|dir| {
        let git_entry = dir.join(".git");
        git_entry.is_file() || (git_entry.is_dir() && git_entry.join("HEAD").is_file())
    })
}

/// Both listings for `folder`, which lies in the repository at `root`: one of
/// the tracked paths, one of the ignored ones, each a single git call.
fn listing(folder: &Path, root: &Path) -> io::Result<Repo> {
    let folder_in_root = folder.strip_prefix(root).map_err(io::Error::other)?;
    let tracked = parse_tracked(&run(folder, &["ls-files", "-z"])?);
    let status = run(
        folder,
        &[
            "--no-optional-locks",
            "status",
            "--porcelain=v1",
            "--ignored",
            "-z",
            "--no-renames",
            "--",
            ".",
        ],
    )?;
    Ok(Repo::Listed {
        tracked,
        ignored: parse_ignored(&status, folder_in_root),
    })
}

/// The output of one git call in `folder`, or why it failed.
fn run(folder: &Path, args: &[&str]) -> io::Result<Vec<u8>> {
    let started = Instant::now();
    let output = git(folder, args, &|| started.elapsed() >= LISTING_TIME)?;
    if output.status.success() {
        return Ok(output.stdout);
    }
    Err(io::Error::other(
        String::from_utf8_lossy(&output.stderr).trim().to_owned(),
    ))
}

/// The first path component of each tracked path git lists, NUL-separated: a
/// tracked file's own name, or the top folder a tracked file lies in.
pub(super) fn parse_tracked(output: &[u8]) -> HashSet<String> {
    output
        .split(|byte| *byte == 0)
        .filter_map(|path| {
            let text = String::from_utf8_lossy(path);
            text.split('/')
                .next()
                .filter(|first| !first.is_empty())
                .map(str::to_owned)
        })
        .collect()
}

/// The names of the ignored entries directly inside `folder_in_root`, from
/// `git status --porcelain=v1 -z`, whose paths are relative to the repository root.
pub(super) fn parse_ignored(output: &[u8], folder_in_root: &Path) -> HashSet<String> {
    output
        .split(|byte| *byte == 0)
        .filter_map(|record| record.strip_prefix(b"!! "))
        .filter_map(|path| {
            let text = String::from_utf8_lossy(path);
            let below = Path::new(text.trim_end_matches('/'))
                .strip_prefix(folder_in_root)
                .ok()?;
            let mut parts = below.components();
            let only = parts.next()?;
            parts
                .next()
                .is_none()
                .then(|| only.as_os_str().to_string_lossy().into_owned())
        })
        .collect()
}

#[cfg(test)]
mod tests;
