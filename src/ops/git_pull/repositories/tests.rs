//! Purpose: Exercise fast-forward pulls and failure cases against local bare remotes.

use super::super::fixtures::{commit, git, text, Fixture};
use super::*;
use std::fs;

#[test]
fn updates_the_current_branch_and_reports_a_second_pull_as_current() {
    let fixture = Fixture::new();
    let repo = fixture.clone_repo("repo with spaces & punctuation");
    fs::write(repo.join("untracked.txt"), "keep").expect("untracked file");
    fixture.advance();
    assert_eq!(pull(&repo, &|| false).outcome, Outcome::Updated);
    assert_eq!(
        fs::read_to_string(repo.join("data.txt")).expect("file"),
        "remote update\n"
    );
    assert_eq!(
        fs::read_to_string(repo.join("untracked.txt")).expect("file"),
        "keep"
    );
    assert_eq!(pull(&repo, &|| false).outcome, Outcome::UpToDate);
    assert_eq!(git(&repo, &["branch", "--show-current"]), "main");
}

#[test]
fn overlapping_local_edits_are_preserved_even_with_autostash_configured() {
    let fixture = Fixture::new();
    let repo = fixture.clone_repo("dirty");
    git(&repo, &["config", "merge.autoStash", "true"]);
    git(&repo, &["config", "rebase.autoStash", "true"]);
    git(&repo, &["config", "pull.rebase", "true"]);
    fs::write(repo.join("data.txt"), "local edit\n").expect("local file");
    let before = git(&repo, &["rev-parse", "HEAD"]);
    fixture.advance();
    assert_eq!(pull(&repo, &|| false).outcome, Outcome::Failed);
    assert_eq!(
        fs::read_to_string(repo.join("data.txt")).expect("file"),
        "local edit\n"
    );
    assert_eq!(git(&repo, &["rev-parse", "HEAD"]), before);
    assert_eq!(git(&repo, &["stash", "list"]), "");
}

#[test]
fn diverged_branches_are_not_merged_or_rebased() {
    let fixture = Fixture::new();
    let repo = fixture.clone_repo("diverged");
    fs::write(repo.join("local.txt"), "local commit").expect("file");
    commit(&repo, "local change");
    let before = git(&repo, &["rev-parse", "HEAD"]);
    fixture.advance();
    assert_eq!(pull(&repo, &|| false).outcome, Outcome::Failed);
    assert_eq!(git(&repo, &["rev-parse", "HEAD"]), before);
    assert_eq!(git(&repo, &["rev-list", "--merges", "HEAD"]), "");
}

#[test]
fn cancelled_and_invalid_repositories_do_not_start_a_pull() {
    let fixture = Fixture::new();
    let repo = fixture.clone_repo("cancelled");
    fixture.advance();
    assert_eq!(pull(&repo, &|| true).outcome, Outcome::Cancelled);
    assert_eq!(
        fs::read_to_string(repo.join("data.txt")).expect("file"),
        "original\n"
    );
    assert_eq!(
        pull(&fixture.root.join("missing"), &|| false).outcome,
        Outcome::Failed
    );
}

#[test]
fn submodules_keep_the_revision_chosen_by_their_parent() {
    let fixture = Fixture::new();
    let parent = fixture.clone_repo("parent");
    git(
        &parent,
        &[
            "-c",
            "protocol.file.allow=always",
            "submodule",
            "add",
            text(&fixture.remote),
            "child",
        ],
    );
    let child = parent.join("child");
    let before = git(&child, &["rev-parse", "HEAD"]);
    fixture.advance();
    assert_eq!(pull(&child, &|| false).outcome, Outcome::Skipped);
    assert_eq!(git(&child, &["rev-parse", "HEAD"]), before);
}

#[test]
fn a_linked_worktree_updates_its_own_current_branch() {
    let fixture = Fixture::new();
    let primary = fixture.clone_repo("primary");
    let linked = fixture.root.join("linked");
    git(
        &primary,
        &[
            "worktree",
            "add",
            "--track",
            "-b",
            "linked",
            text(&linked),
            "origin/main",
        ],
    );
    fixture.advance();
    assert!(linked.join(".git").is_file());
    assert_eq!(pull(&linked, &|| false).outcome, Outcome::Updated);
    assert_eq!(git(&linked, &["branch", "--show-current"]), "linked");
    assert_eq!(
        fs::read_to_string(linked.join("data.txt")).expect("updated file"),
        "remote update\n"
    );
    assert_eq!(
        fs::read_to_string(primary.join("data.txt")).expect("primary file"),
        "original\n"
    );
}
