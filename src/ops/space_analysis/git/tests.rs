//! Unit tests: the two git listings are read into per-folder names, each
//! entry's facts follow from them, and a folder outside git has none.

use super::*;
use std::fs;
use tempfile::TempDir;

fn names(values: &[&str]) -> HashSet<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

fn listed(tracked: &[&str], ignored: &[&str]) -> Repo {
    Repo::Listed {
        tracked: names(tracked),
        ignored: names(ignored),
    }
}

#[test]
fn the_tracked_listing_names_the_first_component_of_each_path() {
    let output = b"src/main.rs\0target/keep.txt\0README.md\0";
    assert_eq!(
        parse_tracked(output),
        names(&["src", "target", "README.md"])
    );
    assert!(parse_tracked(b"").is_empty());
}

#[test]
fn the_ignored_listing_keeps_only_the_entries_directly_inside_the_folder() {
    let output =
        b"!! sub/target/\0!! sub/deep/x.log\0!! sub/notes.log\0?? sub/new.txt\0!! other/z\0";
    assert_eq!(
        parse_ignored(output, Path::new("sub")),
        names(&["target", "notes.log"])
    );
}

#[test]
fn the_ignored_listing_of_the_repository_root_names_its_top_level_entries() {
    let output = b"!! target/\0!! sub/x\0";
    assert_eq!(parse_ignored(output, Path::new("")), names(&["target"]));
}

#[test]
fn an_entry_takes_its_facts_from_the_listings() {
    let repo = listed(&["src"], &["target"]);
    assert_eq!(
        repo.facts("src"),
        GitFacts::Entry {
            tracking: Tracking::Tracked,
            ignoring: Ignoring::NotIgnored,
        }
    );
    assert_eq!(
        repo.facts("target"),
        GitFacts::Entry {
            tracking: Tracking::Untracked,
            ignoring: Ignoring::Ignored,
        }
    );
    assert_eq!(
        repo.facts("notes.md"),
        GitFacts::Entry {
            tracking: Tracking::Untracked,
            ignoring: Ignoring::NotIgnored,
        }
    );
}

#[test]
fn a_folder_with_no_repository_has_no_git_facts() {
    let dir = TempDir::new().expect("temp dir");
    let repo = survey(dir.path());
    assert!(
        matches!(repo, Repo::Outside),
        "no `.git` above {:?}",
        dir.path()
    );
    assert_eq!(repo.facts("anything"), GitFacts::Outside);
    assert!(repo
        .note()
        .is_some_and(|note| note.contains("Not inside a git repository")));
}

#[test]
fn a_folder_inside_a_git_directory_is_marked_as_git_metadata() {
    let dir = TempDir::new().expect("temp dir");
    let inside = dir.path().join(".git").join("objects");
    fs::create_dir_all(&inside).expect("test setup");
    assert!(matches!(survey(&inside), Repo::InsideGitDir));
    assert_eq!(survey(&inside).facts("ab"), GitFacts::InsideGitDir);
    assert!(survey(&inside)
        .note()
        .is_some_and(|note| note.contains("Inside a .git folder")));
}

#[test]
fn a_repository_that_git_cannot_list_is_unreadable_and_says_so() {
    let dir = TempDir::new().expect("temp dir");
    fs::write(
        dir.path().join(".git"),
        "gitdir: /definitely/not/a/git/dir\n",
    )
    .expect("test setup");
    let repo = survey(dir.path());
    assert!(matches!(repo, Repo::Unreadable), "{:?}", repo.note());
    assert_eq!(repo.facts("target"), GitFacts::Unreadable);
    assert!(repo
        .note()
        .is_some_and(|note| note.contains("could not list")));
}

#[test]
fn a_listed_repository_has_nothing_to_say_about_itself() {
    assert_eq!(listed(&[], &[]).note(), None);
}
