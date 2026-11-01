//! Unit tests: judging a folder inside a real repository, and one git cannot list,
//! with the git CLI run on temporary folders.

use super::tests::{git, unreviewed, verdict_of, write, ROOMY};
use super::*;
use crate::space_verdict::{Keep, Regenerable, ReviewReason, Verdict};
use std::fs;
use tempfile::TempDir;

/// A repository with a tracked source file, git-ignored `target/` and `*.log`
/// entries, and an untracked note.
fn repository_with_build_output() -> TempDir {
    let dir = TempDir::new().expect("temp dir");
    git(dir.path(), &["init", "-q"]);
    fs::write(dir.path().join(".gitignore"), "target/\n*.log\n").expect("test setup");
    write(dir.path(), "src/main.rs", 10);
    write(dir.path(), "target/debug/out.o", 500);
    write(dir.path(), "debug.log", 20);
    write(dir.path(), "notes.md", 30);
    git(dir.path(), &["add", ".gitignore", "src/main.rs"]);
    dir
}

#[test]
fn a_repository_keeps_tracked_files_and_marks_ignored_build_output_safe() {
    let dir = repository_with_build_output();
    let dialog = scan(dir.path(), ROOMY).expect("scan");
    assert_eq!(verdict_of(&dialog, "src"), Verdict::Keep(Keep::Tracked));
    assert_eq!(
        verdict_of(&dialog, ".gitignore"),
        Verdict::Keep(Keep::Tracked)
    );
    assert_eq!(
        verdict_of(&dialog, "target"),
        Verdict::Safe(Regenerable::CargoTarget)
    );
    assert_eq!(
        verdict_of(&dialog, "debug.log"),
        unreviewed(ReviewReason::IgnoredNotCache)
    );
    assert_eq!(
        verdict_of(&dialog, "notes.md"),
        unreviewed(ReviewReason::NotIgnored)
    );
    assert_eq!(dialog.reclaimable.bytes(), 500, "the target folder alone");
    assert_eq!(dialog.note, "", "a listed repository needs no note");
}

/// Git ignores `target/`, yet a repository vendored inside it is someone's
/// work: the folder is kept, and nothing counts toward the reclaimable total.
#[test]
fn an_ignored_folder_holding_a_nested_repository_is_kept() {
    let dir = TempDir::new().expect("temp dir");
    git(dir.path(), &["init", "-q"]);
    fs::write(dir.path().join(".gitignore"), "target/\n").expect("test setup");
    git(dir.path(), &["add", ".gitignore"]);
    write(dir.path(), "target/vendor/.git/HEAD", 4);
    write(dir.path(), "target/out.o", 10);
    let dialog = scan(dir.path(), ROOMY).expect("scan");
    assert_eq!(verdict_of(&dialog, "target"), Verdict::Keep(Keep::HoldsGit));
    assert_eq!(dialog.reclaimable.bytes(), 0);
}

#[test]
fn a_repository_git_cannot_list_makes_nothing_safe() {
    let dir = TempDir::new().expect("temp dir");
    fs::write(
        dir.path().join(".git"),
        "gitdir: /definitely/not/a/git/dir\n",
    )
    .expect("test setup");
    write(dir.path(), "__pycache__/mod.pyc", 40);
    write(dir.path(), "target/out.o", 30);
    let dialog = scan(dir.path(), ROOMY).expect("scan");
    assert_eq!(
        verdict_of(&dialog, "__pycache__"),
        unreviewed(ReviewReason::GitUnreadable)
    );
    assert_eq!(
        verdict_of(&dialog, "target"),
        unreviewed(ReviewReason::GitUnreadable)
    );
    assert_eq!(dialog.reclaimable.bytes(), 0);
    assert!(dialog.note.contains("could not list"), "{}", dialog.note);
}

#[test]
fn a_folder_inside_a_git_directory_keeps_everything() {
    let dir = TempDir::new().expect("temp dir");
    write(dir.path(), ".git/objects/ab/cdef", 12);
    let dialog = scan(&dir.path().join(".git/objects"), ROOMY).expect("scan");
    assert_eq!(verdict_of(&dialog, "ab"), Verdict::Keep(Keep::GitMetadata));
    assert!(
        dialog.note.contains("Inside a .git folder"),
        "{}",
        dialog.note
    );
}
