//! Update the current branch and local main, then rebase the topic onto origin/main.

use std::io;
use std::path::{Path, PathBuf};
use std::process::Output;

use super::git_pull::repositories::git;
use crate::models::GitSyncTarget;

const ORIGIN_MAIN: &str = "refs/remotes/origin/main";
const LOCAL_MAIN: &str = "refs/heads/main";
const PULL_UPSTREAM: &[&str] = &[
    "-c",
    "rebase.updateRefs=false",
    "pull",
    "--rebase",
    "--no-autostash",
    "--no-recurse-submodules",
];
const FETCH_MAIN: &[&str] = &[
    "fetch",
    "--no-tags",
    "--no-prune",
    "--no-recurse-submodules",
    "--refmap=",
    "origin",
    "+refs/heads/main:refs/remotes/origin/main",
];
const STATUS: &[&str] = &[
    "status",
    "--porcelain=v1",
    "-z",
    "--untracked-files=normal",
    "--ignore-submodules=none",
];

pub(super) fn inspect(directory: &Path) -> io::Result<GitSyncTarget> {
    let root = git_path(directory, "--show-toplevel")?.canonicalize()?;
    let git_dir = git_path(&root, "--absolute-git-dir")?;
    check_operation(&git_dir)?;
    let head = checked(&root, &["symbolic-ref", "HEAD"])?;
    let branch = std::str::from_utf8(&head.stdout)
        .map_err(io::Error::other)?
        .trim_end_matches(['\r', '\n'])
        .strip_prefix("refs/heads/")
        .ok_or_else(|| io::Error::other("Select a local branch before syncing"))?
        .to_owned();
    checked(&root, &["rev-parse", "--verify", "HEAD"])?;
    if !checked(&root, STATUS)?.stdout.is_empty() {
        return Err(io::Error::other(
            "Commit or stash working tree changes before syncing",
        ));
    }
    Ok(GitSyncTarget { root, branch })
}

fn check_operation(git_dir: &Path) -> io::Result<()> {
    for marker in [
        "MERGE_HEAD",
        "CHERRY_PICK_HEAD",
        "REVERT_HEAD",
        "BISECT_START",
        "sequencer",
        "rebase-apply",
        "rebase-merge",
    ] {
        if git_dir.join(marker).try_exists()? {
            return Err(io::Error::other(format!(
                "Finish the Git operation in progress ({marker}) before syncing"
            )));
        }
    }
    Ok(())
}

fn check_target(target: &GitSyncTarget) -> io::Result<()> {
    let confirmed = inspect(&target.root)?;
    if confirmed.root != target.root || confirmed.branch != target.branch {
        return Err(io::Error::other(
            "The repository or branch changed after confirmation",
        ));
    }
    Ok(())
}

pub fn run(target: &GitSyncTarget) -> io::Result<String> {
    check_target(target)?;
    let repo = &target.root;
    if target.branch != "main" {
        checked(repo, PULL_UPSTREAM)?;
    }
    checked(repo, FETCH_MAIN)?;
    check_target(target)?;
    if target.branch == "main" {
        checked(repo, &["merge", "--ff-only", "--no-autostash", ORIGIN_MAIN])?;
        return Ok("Updated main from origin/main".to_owned());
    }
    update_main(repo)?;
    checked(
        repo,
        &["rebase", "--no-update-refs", "--no-autostash", ORIGIN_MAIN],
    )?;
    Ok(format!(
        "Updated main and rebased {} onto origin/main",
        target.branch
    ))
}

fn update_main(repo: &Path) -> io::Result<()> {
    // An ahead-only main already includes every remote commit.
    if git(
        repo,
        &["merge-base", "--is-ancestor", ORIGIN_MAIN, LOCAL_MAIN],
        &|| false,
    )?
    .status
    .success()
    {
        return Ok(());
    }
    // Git checks for non-fast-forwards and branches in use by any worktree.
    checked(
        repo,
        &[
            "fetch",
            "--no-tags",
            "--no-prune",
            "--no-recurse-submodules",
            "--no-write-fetch-head",
            ".",
            "refs/remotes/origin/main:refs/heads/main",
        ],
    )?;
    Ok(())
}

fn checked(repo: &Path, args: &[&str]) -> io::Result<Output> {
    let output = git(repo, args, &|| false)?;
    if !output.status.success() {
        return Err(io::Error::other(format!(
            "git {} failed: {}{}",
            args.join(" "),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        )));
    }
    Ok(output)
}

fn git_path(repo: &Path, flag: &str) -> io::Result<PathBuf> {
    let mut bytes = checked(repo, &["rev-parse", flag])?.stdout;
    bytes.truncate(bytes.strip_suffix(b"\n").map_or(bytes.len(), <[u8]>::len));
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;
        Ok(std::ffi::OsString::from_vec(bytes).into())
    }
    #[cfg(not(unix))]
    Ok(String::from_utf8(bytes)
        .map_err(io::Error::other)?
        .trim_end_matches('\r')
        .into())
}

#[cfg(test)]
mod tests;
