//! Purpose: Git-ignored entries and the `.git` entry of a repository are marked;
//! a tracked file is not, even when a rule matches it.

use std::fs;
use std::process::Command;

use tempfile::TempDir;

use super::load_ignored;
use crate::models::PanelMetadata;
use crate::values::entry::DirEntry;

fn entry(name: &str) -> DirEntry {
    DirEntry {
        name: name.to_owned(),
        ..DirEntry::default()
    }
}

/// A repository with a `.gitignore` that ignores `*.log` and `target/`.
fn repository() -> TempDir {
    let repo = TempDir::new().expect("temp dir");
    let status = Command::new("git")
        .arg("init")
        .arg("--quiet")
        .arg(repo.path())
        .status()
        .expect("git init");
    assert!(status.success(), "git init failed");
    fs::write(repo.path().join(".gitignore"), "*.log\ntarget/\n").expect("gitignore");
    fs::write(repo.path().join("a.log"), "log").expect("log");
    fs::write(repo.path().join("main.rs"), "fn main() {}").expect("source");
    fs::create_dir(repo.path().join("target")).expect("target");
    repo
}

fn listing() -> [DirEntry; 5] {
    [".git", ".gitignore", "a.log", "main.rs", "target"].map(entry)
}

#[test]
fn ignored_entries_and_the_git_directory_are_marked() {
    let repo = repository();
    let mut metadata = PanelMetadata::default();
    load_ignored(repo.path(), &listing(), &mut metadata);
    assert!(metadata.ignored.contains(".git"));
    assert!(metadata.ignored.contains("a.log"));
    assert!(metadata.ignored.contains("target"));
    assert!(!metadata.ignored.contains("main.rs"));
    assert!(!metadata.ignored.contains(".gitignore"));
}

#[test]
fn a_tracked_file_is_not_marked_even_when_a_rule_matches() {
    let repo = repository();
    let status = Command::new("git")
        .arg("-C")
        .arg(repo.path())
        .args(["add", "-f", "a.log"])
        .status()
        .expect("git add");
    assert!(status.success(), "git add failed");
    let mut metadata = PanelMetadata::default();
    load_ignored(repo.path(), &listing(), &mut metadata);
    assert!(!metadata.ignored.contains("a.log"));
    assert!(metadata.ignored.contains("target"));
}

#[test]
fn the_repository_facts_mark_the_ignored_entries() {
    let repo = repository();
    let mut metadata = PanelMetadata::default();
    crate::git::load_repo_facts(repo.path(), &listing(), &mut metadata);
    assert!(metadata.ignored.contains("target"));
    assert!(metadata.ignored.contains(".git"));
    assert!(!metadata.ignored.contains("main.rs"));
}
