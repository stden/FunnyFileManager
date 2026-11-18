//! Exercise branch synchronization against remotes in a temporary directory.

use super::*;
use crate::ops::git_pull::tests_fixtures::{commit, git, text, Fixture};
use std::fs;

fn topic(fixture: &Fixture) -> PathBuf {
    let repo = fixture.clone_repo("local");
    git(&repo, &["config", "user.name", "Test User"]);
    git(&repo, &["config", "user.email", "test@example.com"]);
    git(&repo, &["switch", "-c", "topic"]);
    git(&repo, &["push", "-u", "origin", "topic"]);
    repo
}

fn write_commit(repo: &Path, file: &str) {
    fs::write(repo.join(file), file).expect("write content");
    commit(repo, file);
}

#[test]
fn pulls_topic_then_rebases_its_commits_on_fresh_main() {
    let fixture = Fixture::new();
    let repo = topic(&fixture);
    git(&fixture.seed, &["switch", "-c", "topic"]);
    write_commit(&fixture.seed, "remote-topic");
    git(&fixture.seed, &["push", text(&fixture.remote), "topic"]);
    git(&fixture.seed, &["switch", "main"]);
    fixture.advance();
    write_commit(&repo, "local-topic");
    let nested = repo.join("empty subdirectory");
    fs::create_dir(&nested).expect("directory");
    let target = inspect(&nested).expect("inspect");
    assert_eq!(target.branch, "topic");
    run(&target).expect("sync");
    assert_eq!(git(&repo, &["branch", "--show-current"]), "topic");
    assert_eq!(
        git(&repo, &["rev-parse", "main"]),
        git(&fixture.seed, &["rev-parse", "HEAD"])
    );
    assert_eq!(git(&repo, &["rev-list", "--count", "main..HEAD"]), "2");
    git(&repo, &["merge-base", "--is-ancestor", "main", "HEAD"]);
    assert!(repo.join("remote-topic").is_file() && repo.join("local-topic").is_file());
}

#[test]
fn main_fast_forwards_and_a_missing_local_main_is_created() {
    let fixture = Fixture::new();
    let repo = fixture.clone_repo("local");
    fixture.advance();
    run(&inspect(&repo).expect("on main")).expect("fast-forward main");
    let expected = git(&repo, &["rev-parse", "HEAD"]);
    git(&repo, &["switch", "-c", "topic", "--track", "origin/main"]);
    git(&repo, &["branch", "-D", "main"]);
    run(&inspect(&repo).expect("on topic")).expect("recreate main");
    assert_eq!(git(&repo, &["rev-parse", "main"]), expected);
}

#[test]
fn refuses_diverged_main_and_another_worktrees_main() {
    let fixture = Fixture::new();
    let repo = topic(&fixture);
    let other = fixture.root.join("second worktree");
    git(&repo, &["worktree", "add", text(&other), "main"]);
    fixture.advance();
    let target = inspect(&repo).expect("on topic");
    let previous = git(&repo, &["rev-parse", "main"]);
    assert!(run(&target).is_err());
    assert_eq!(git(&repo, &["rev-parse", "main"]), previous);
    write_commit(&other, "diverged");
    let diverged = git(&other, &["rev-parse", "HEAD"]);
    git(&repo, &["worktree", "remove", text(&other)]);
    assert!(run(&target).is_err());
    assert_eq!(git(&repo, &["rev-parse", "main"]), diverged);
}

#[test]
fn refuses_dirty_detached_and_changed_targets() {
    let fixture = Fixture::new();
    let repo = fixture.clone_repo("local");
    let original = inspect(&repo).expect("on main");
    fs::write(repo.join("untracked"), "local work").expect("untracked");
    assert!(inspect(&repo).is_err());
    fs::remove_file(repo.join("untracked")).expect("delete fixture file");
    git(&repo, &["switch", "-c", "other"]);
    assert!(run(&original).is_err());
    git(&repo, &["switch", "--detach"]);
    assert!(inspect(&repo).is_err());
}

#[test]
fn rebase_conflict_keeps_the_conflicting_files_and_git_state() {
    let fixture = Fixture::new();
    let repo = topic(&fixture);
    write_commit(&repo, "data.txt");
    fixture.advance();
    let err = run(&inspect(&repo).expect("on topic")).expect_err("conflict");
    assert!(err.to_string().contains("rebase"));
    assert_eq!(
        git(&repo, &["diff", "--name-only", "--diff-filter=U"]),
        "data.txt"
    );
    assert!(inspect(&repo).is_err());
}

#[test]
fn missing_remote_main_never_rebases_onto_the_stale_tracking_ref() {
    let fixture = Fixture::new();
    let repo = topic(&fixture);
    git(&fixture.remote, &["update-ref", "-d", "refs/heads/main"]);
    let before = git(&repo, &["rev-parse", "HEAD"]);
    let error = run(&inspect(&repo).expect("on topic")).expect_err("missing main");
    assert!(error.to_string().contains("refs/heads/main"));
    assert_eq!(git(&repo, &["rev-parse", "HEAD"]), before);
}

#[test]
fn an_unpublished_topic_reports_that_its_upstream_is_missing() {
    let fixture = Fixture::new();
    let repo = fixture.clone_repo("local");
    git(&repo, &["switch", "-c", "unpublished"]);
    let before = git(&repo, &["rev-parse", "HEAD"]);
    let error = run(&inspect(&repo).expect("on local topic")).expect_err("no upstream");
    assert!(error.to_string().contains("pull"));
    assert_eq!(git(&repo, &["rev-parse", "HEAD"]), before);
}
