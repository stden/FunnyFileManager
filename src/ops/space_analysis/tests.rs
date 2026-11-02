//! Unit tests: judging a folder's entries against the disk and git, with the
//! limits and the note they produce. A real `git` runs on temporary folders.

use super::*;
use crate::space_verdict::{Regenerable, ReviewReason, SizeNote, Verdict};
use std::fs;
use std::process::Command;
use tempfile::TempDir;

pub(super) const ROOMY: Limits = Limits {
    entries: 100_000,
    time: Duration::from_secs(60),
};

pub(super) fn write(root: &Path, relative: &str, bytes: usize) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().expect("a file has a parent")).expect("test setup");
    fs::write(path, vec![b'x'; bytes]).expect("test setup");
}

pub(super) fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(dir)
        .status()
        .expect("git runs");
    assert!(status.success(), "git {args:?}");
}

pub(super) fn verdict_of(dialog: &SpaceAnalysisDialog, name: &str) -> Verdict {
    dialog.rows.iter().find(|row| row.name == name).map_or_else(
        || panic!("no row named {name}: {:?}", dialog.rows),
        |row| row.verdict,
    )
}

pub(super) fn unreviewed(reason: ReviewReason) -> Verdict {
    Verdict::Review {
        reason,
        size: SizeNote::Ordinary,
    }
}

#[test]
fn a_folder_outside_git_keeps_only_its_caches_safe() {
    let dir = TempDir::new().expect("temp dir");
    write(dir.path(), "__pycache__/mod.pyc", 40);
    write(dir.path(), "target/out.o", 30);
    write(dir.path(), "notes.txt", 20);
    let dialog = scan(dir.path(), ROOMY).expect("scan");
    assert_eq!(
        verdict_of(&dialog, "__pycache__"),
        Verdict::Safe(Regenerable::PythonBytecode)
    );
    assert_eq!(
        verdict_of(&dialog, "target"),
        unreviewed(ReviewReason::NotUnderGit)
    );
    assert_eq!(
        verdict_of(&dialog, "notes.txt"),
        unreviewed(ReviewReason::NotUnderGit)
    );
    assert_eq!(dialog.reclaimable.bytes(), 40, "only the cache is safe");
    assert!(
        dialog.note.contains("Not inside a git repository"),
        "{}",
        dialog.note
    );
}

#[test]
fn the_entry_budget_stops_the_scan_and_the_note_names_it() {
    let dir = TempDir::new().expect("temp dir");
    for index in 0..6 {
        write(dir.path(), &format!("f{index}.bin"), 1);
    }
    let limits = Limits {
        entries: 3,
        time: Duration::from_secs(60),
    };
    let dialog = scan(dir.path(), limits).expect("scan");
    assert_eq!(dialog.rows.len(), 3);
    assert!(dialog.note.contains("entry budget"), "{}", dialog.note);
}

#[test]
fn the_time_budget_stops_the_scan_and_the_note_names_it() {
    let dir = TempDir::new().expect("temp dir");
    write(dir.path(), "f.bin", 1);
    let limits = Limits {
        entries: 100_000,
        time: Duration::ZERO,
    };
    let dialog = scan(dir.path(), limits).expect("scan");
    assert_eq!(dialog.rows.len(), 0);
    assert!(dialog.note.contains("time budget"), "{}", dialog.note);
}

#[test]
fn a_folder_that_cannot_be_listed_is_an_error() {
    assert!(scan(Path::new("/definitely/not/here"), ROOMY).is_err());
}

#[test]
fn the_scan_removes_nothing() {
    let dir = TempDir::new().expect("temp dir");
    write(dir.path(), "__pycache__/mod.pyc", 4);
    write(dir.path(), "notes.txt", 4);
    let _dialog = scan(dir.path(), ROOMY).expect("scan");
    assert!(dir.path().join("__pycache__/mod.pyc").is_file());
    assert!(dir.path().join("notes.txt").is_file());
}

#[test]
fn each_stop_names_its_own_limit() {
    assert!(stop_note(Stop::Entries).contains("entry budget"));
    assert!(stop_note(Stop::Time).contains("time budget"));
}

#[test]
fn the_note_joins_what_limited_the_answer() {
    assert!(!note(None, 0, &git::Repo::Outside).contains("budget"));
    let both = note(Some(Stop::Entries), 2, &git::Repo::Outside);
    assert!(both.contains("entry budget"), "{both}");
    assert!(both.contains("2 folder(s) could not be read"), "{both}");
    assert!(both.contains("Not inside a git repository"), "{both}");
}
