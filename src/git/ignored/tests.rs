//! Purpose: Git-ignored entries and the `.git` entry of a repository are marked;
//! a tracked file is not, even when a rule matches it.

use std::fs;
use std::process::Command;

use tempfile::TempDir;

use super::{load_ignored, CHECK_IGNORE_RUNS};
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

#[test]
fn repeated_load_ignored_reuses_cache_until_invalidation() {
    let repo = repository();
    let list = listing();
    CHECK_IGNORE_RUNS.with(|runs| runs.set(0));

    let mut first = PanelMetadata::default();
    load_ignored(repo.path(), &list, &mut first);
    assert_eq!(CHECK_IGNORE_RUNS.with(std::cell::Cell::get), 1);
    assert!(first.ignored.contains("a.log"));

    let mut second = PanelMetadata::default();
    load_ignored(repo.path(), &list, &mut second);
    assert_eq!(CHECK_IGNORE_RUNS.with(std::cell::Cell::get), 1);
    assert_eq!(first.ignored, second.ignored);

    crate::git::invalidate_repo_cache_for(repo.path());
    let mut refreshed = PanelMetadata::default();
    load_ignored(repo.path(), &list, &mut refreshed);
    assert_eq!(CHECK_IGNORE_RUNS.with(std::cell::Cell::get), 2);
}

#[test]
fn ancestor_gitignore_update_refreshes_root_and_nested_directory_caches() {
    let repo = repository();
    let list = listing();
    let sub = repo.path().join("sub");
    fs::create_dir(&sub).expect("sub dir");
    fs::write(sub.join("b.log"), "log").expect("sub log");
    let sub_list = [entry("b.log")];
    CHECK_IGNORE_RUNS.with(|runs| runs.set(0));

    let mut sub_first = PanelMetadata::default();
    load_ignored(&sub, &sub_list, &mut sub_first);
    load_ignored(&sub, &sub_list, &mut sub_first);
    assert_eq!(CHECK_IGNORE_RUNS.with(std::cell::Cell::get), 1);
    assert!(sub_first.ignored.contains("b.log"));

    fs::write(repo.path().join(".gitignore"), "target/\n").expect("update gitignore");
    let mut root_refreshed = PanelMetadata::default();
    load_ignored(repo.path(), &list, &mut root_refreshed);
    assert!(!root_refreshed.ignored.contains("a.log"));

    let mut sub_refreshed = PanelMetadata::default();
    load_ignored(&sub, &sub_list, &mut sub_refreshed);
    assert_eq!(CHECK_IGNORE_RUNS.with(std::cell::Cell::get), 3);
    assert!(!sub_refreshed.ignored.contains("b.log"));
}

#[test]
fn empty_listing_and_non_repo_skip_git_check_ignore() {
    let repo = repository();
    let outside = TempDir::new().expect("outside");
    CHECK_IGNORE_RUNS.with(|runs| runs.set(0));

    let mut empty_meta = PanelMetadata::default();
    load_ignored(repo.path(), &[], &mut empty_meta);
    load_ignored(outside.path(), &[entry("a.log")], &mut empty_meta);
    assert_eq!(CHECK_IGNORE_RUNS.with(std::cell::Cell::get), 0);
    assert!(empty_meta.ignored.is_empty());

    let mut git_only = PanelMetadata::default();
    load_ignored(repo.path(), &[entry(".git")], &mut git_only);
    assert_eq!(CHECK_IGNORE_RUNS.with(std::cell::Cell::get), 0);
    assert!(git_only.ignored.contains(".git"));

    let stamp =
        super::cache::IgnoreStamp::capture(outside.path(), outside.path(), outside.path(), &[]);
    for i in 0..=256 {
        super::cache::store(&outside.path().join(format!("d{i}")), stamp, &[]);
    }
    assert_eq!(
        super::cache::lookup(&outside.path().join("d255"), stamp),
        Some(Vec::new())
    );
    assert_eq!(
        super::cache::lookup(&outside.path().join("d256"), stamp),
        Some(Vec::new())
    );
}

#[test]
fn failed_spawn_and_hung_check_ignore_fall_back_cleanly() {
    let repo = repository();
    let invalid_dir = std::path::Path::new("bad\0dir");
    assert_eq!(
        super::ignored_names(
            invalid_dir,
            repo.path(),
            &repo.path().join(".git"),
            &["a.log"]
        ),
        Vec::<String>::new()
    );

    #[cfg(unix)]
    let mut child = Command::new("sleep")
        .arg("10")
        .spawn()
        .expect("spawn sleep");
    #[cfg(windows)]
    let mut child = Command::new("ping.exe")
        .args(["-n", "6", "127.0.0.1"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("spawn ping");
    assert_eq!(
        super::wait_for_exit(&mut child, std::time::Duration::from_millis(20)),
        None
    );
}
