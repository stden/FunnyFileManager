//! Unit tests: reading a branch out of `.git/HEAD`.

use super::*;
use tempfile::TempDir;

/// A repository whose `HEAD` says exactly `head`.
fn repo(head: &str) -> TempDir {
    let dir = TempDir::new().expect("temp dir");
    let git = dir.path().join(".git");
    fs::create_dir(&git).expect("git dir");
    fs::write(git.join("HEAD"), head).expect("HEAD");
    dir
}

#[test]
fn a_symbolic_head_names_the_branch() {
    let dir = repo("ref: refs/heads/main\n");
    assert_eq!(branch(dir.path()).as_deref(), Some("main"));
}

/// `refs/heads/feature/x` is the branch `feature/x` — only the known prefix
/// comes off, not everything up to the last slash.
#[test]
fn a_branch_may_contain_slashes() {
    let dir = repo("ref: refs/heads/feature/nested/thing\n");
    assert_eq!(branch(dir.path()).as_deref(), Some("feature/nested/thing"));
}

#[test]
fn a_detached_head_shows_a_short_hash() {
    let dir = repo("9f2a1c4e8b7d6f5a4c3b2a1908f7e6d5c4b3a291\n");
    assert_eq!(branch(dir.path()).as_deref(), Some("9f2a1c4"));
}

/// The branch is looked for upwards, since the panel is usually somewhere
/// inside the repository rather than at its root.
#[test]
fn a_subdirectory_reports_the_repository_branch() {
    let dir = repo("ref: refs/heads/main\n");
    let nested = dir.path().join("src/ui/deep");
    fs::create_dir_all(&nested).expect("nested");
    assert_eq!(branch(&nested).as_deref(), Some("main"));
}

/// In a worktree or a submodule `.git` is a *file* pointing elsewhere.
/// Following it is the difference between naming the branch and reporting
/// nothing at all.
#[test]
fn a_gitdir_pointer_file_is_followed() {
    let dir = TempDir::new().expect("temp dir");
    let real = dir.path().join("real-git-dir");
    fs::create_dir(&real).expect("real");
    fs::write(real.join("HEAD"), "ref: refs/heads/worktree-branch\n").expect("HEAD");

    let work = dir.path().join("worktree");
    fs::create_dir(&work).expect("work");
    fs::write(work.join(".git"), format!("gitdir: {}\n", real.display())).expect("pointer");

    assert_eq!(branch(&work).as_deref(), Some("worktree-branch"));
}

#[test]
fn a_relative_gitdir_pointer_is_resolved_against_its_own_directory() {
    let dir = TempDir::new().expect("temp dir");
    let work = dir.path().join("worktree");
    fs::create_dir_all(work.join("elsewhere")).expect("dirs");
    fs::write(work.join("elsewhere/HEAD"), "ref: refs/heads/relative\n").expect("HEAD");
    fs::write(work.join(".git"), "gitdir: elsewhere\n").expect("pointer");

    assert_eq!(branch(&work).as_deref(), Some("relative"));
}

#[test]
fn a_directory_outside_any_repository_has_no_branch() {
    let dir = TempDir::new().expect("temp dir");
    assert_eq!(branch(dir.path()), None);
}

/// A `.git` that exists but holds nothing readable must answer "no branch"
/// rather than showing a fragment of a file.
#[test]
fn an_unreadable_head_is_not_guessed_at() {
    assert_eq!(head_label(""), None);
    assert_eq!(head_label("not a ref and not a hash"), None);
    // Too short to be a hash.
    assert_eq!(head_label("9f2a1c"), None);

    let dir = TempDir::new().expect("temp dir");
    fs::create_dir(dir.path().join(".git")).expect("git dir");
    assert_eq!(branch(dir.path()), None, "no HEAD file at all");
}

/// A real `.git` directory laid out the way git lays one out, so the happy
/// path is exercised against something more than a two-line fixture.
#[test]
fn a_realistic_repository_layout_reports_its_branch() {
    let dir = TempDir::new().expect("temp dir");
    let git = dir.path().join(".git");
    fs::create_dir_all(git.join("refs/heads")).expect("refs");
    fs::create_dir_all(git.join("objects")).expect("objects");
    fs::write(git.join("HEAD"), "ref: refs/heads/main\n").expect("HEAD");
    fs::write(git.join("refs/heads/main"), "0123456789abcdef\n").expect("ref");
    fs::write(git.join("config"), "[core]\n\tbare = false\n").expect("config");

    assert_eq!(branch(dir.path()).as_deref(), Some("main"));
}

/// The nearest `.git` wins: a repository inside a repository reports its own
/// branch, not its parent's.
#[test]
fn the_nearest_repository_is_the_one_reported() {
    let outer = repo("ref: refs/heads/outer\n");
    let inner = outer.path().join("vendor/inner");
    fs::create_dir_all(inner.join(".git")).expect("inner git");
    fs::write(inner.join(".git/HEAD"), "ref: refs/heads/inner\n").expect("HEAD");

    assert_eq!(branch(&inner).as_deref(), Some("inner"));
}
