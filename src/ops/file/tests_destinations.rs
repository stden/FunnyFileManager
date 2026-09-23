//! Unit tests: where an F5/F6 transfer lands, and what the status line says
//! about it afterwards.

use super::tests_transfer_policies::{app_with, mark, settings};
use super::*;
use crate::copy::{CopyPrompt, ExistingFiles, Purpose};

// ─── onto itself ────────────────────────────────────────────────────────────

/// Both panels on one folder is the dialog's default destination. Far refuses
/// a copy or a move onto itself ("You cannot copy or move a file or folder
/// onto itself") whatever the "already existing files" choice; with Ask, F6
/// used to rename the file to `name_1` and report it as copied.
#[test]
fn a_transfer_onto_itself_is_refused_and_the_file_keeps_its_name() {
    for purpose in [Purpose::Copy, Purpose::Move] {
        let (mut app, from, _to) = app_with(&[("utf16le.txt", b"payload")]);
        mark(&mut app, "utf16le.txt");

        let mut dialog = settings(from.path());
        dialog.purpose = purpose;
        dialog.existing = ExistingFiles::Ask;
        copy_with(&mut app, &dialog);

        assert_eq!(
            fs::read(from.path().join("utf16le.txt")).expect("still there"),
            b"payload",
            "{purpose:?}"
        );
        assert!(
            !from.path().join("utf16le_1.txt").exists(),
            "{purpose:?} made or renamed to a second name"
        );
        let status = app.status.to_string();
        assert!(status.contains("onto itself"), "{purpose:?}: {status}");
    }
}

/// A folder onto itself is the same refusal, not a duplicate `name_1` tree.
#[test]
fn a_folder_onto_itself_is_refused_too() {
    let (mut app, from, _to) = app_with(&[]);
    fs::create_dir(from.path().join("project")).expect("test setup");
    mark(&mut app, "project");

    let mut dialog = settings(from.path());
    dialog.purpose = Purpose::Move;
    copy_with(&mut app, &dialog);

    assert!(from.path().join("project").is_dir());
    assert!(!from.path().join("project_1").exists());
    assert!(app.status.to_string().contains("onto itself"));
}

// ─── the status line ────────────────────────────────────────────────────────

/// F6 says it moved, and a failed F6 says "Moved … error(s)", not "Copied".
#[test]
fn a_move_is_reported_as_a_move() {
    let (mut app, _from, to) = app_with(&[("notes.txt", b"x")]);
    mark(&mut app, "notes.txt");
    mark(&mut app, "vanished.txt");

    let mut dialog = settings(to.path());
    dialog.purpose = Purpose::Move;
    copy_with(&mut app, &dialog);

    let status = app.status.to_string();
    assert!(
        status.starts_with("Moved 1 item(s), 1 error(s)"),
        "{status}"
    );
}

/// "Existing files kept, … renamed" only when something was in the way: a
/// plain transfer into an empty folder renamed nothing.
#[test]
fn renaming_is_mentioned_only_when_it_happened() {
    for purpose in [Purpose::Copy, Purpose::Move] {
        let (mut app, _from, to) = app_with(&[("notes.txt", b"x")]);
        mark(&mut app, "notes.txt");

        let mut dialog = settings(to.path());
        dialog.purpose = purpose;
        dialog.existing = ExistingFiles::Ask;
        copy_with(&mut app, &dialog);

        let status = app.status.to_string();
        let verb = match purpose {
            Purpose::Copy => "Copied to ",
            Purpose::Move => "Moved to ",
        };
        assert!(status.starts_with(verb), "{status}");
        assert!(!status.contains("renamed"), "nothing was renamed: {status}");
    }
}

// ─── what a typed destination means ─────────────────────────────────────────
//
// Far's rule: an existing folder, a name ending in a separator, or the
// destination of several items is a folder to put them in; anything else is
// the new name of the one item. That is how F6 renames — its dialog says
// "Rename or move" and its button says Rename.

/// The dialog as if the user had cleared the field and typed `text`.
fn typed(purpose: Purpose, text: &str) -> CopyPrompt {
    let mut dialog = settings(Path::new("."));
    dialog.purpose = purpose;
    dialog.destination = text.to_owned();
    dialog
}

#[test]
fn f6_to_a_new_name_renames_the_file() {
    let (mut app, from, _to) = app_with(&[("medium.txt", b"body")]);
    mark(&mut app, "medium.txt");

    copy_with(&mut app, &typed(Purpose::Move, "renamed.txt"));

    let renamed = from.path().join("renamed.txt");
    assert!(renamed.is_file(), "a file named renamed.txt, not a folder");
    assert_eq!(fs::read(&renamed).expect("renamed"), b"body");
    assert!(
        !from.path().join("medium.txt").exists(),
        "the old name is gone"
    );
    let status = app.status.to_string();
    assert!(status.starts_with("Moved to "), "{status}");
    assert!(status.ends_with("renamed.txt"), "{status}");
}

#[test]
fn f5_to_a_new_name_copies_the_file_under_that_name() {
    let (mut app, from, _to) = app_with(&[("run.sh", b"#!/bin/sh\n")]);
    mark(&mut app, "run.sh");

    copy_with(&mut app, &typed(Purpose::Copy, "run-copy.sh"));

    let copy = from.path().join("run-copy.sh");
    assert!(copy.is_file(), "a file named run-copy.sh, not a folder");
    assert_eq!(fs::read(&copy).expect("copied"), b"#!/bin/sh\n");
    assert!(from.path().join("run.sh").is_file(), "the original stays");
}

/// A folder sent to a new name *becomes* that folder: Far puts its contents
/// straight into it, not into `new/old`.
#[test]
fn a_folder_sent_to_a_new_name_becomes_that_folder() {
    for purpose in [Purpose::Copy, Purpose::Move] {
        let (mut app, from, _to) = app_with(&[]);
        fs::create_dir_all(from.path().join("project/src")).expect("test setup");
        fs::write(from.path().join("project/src/main.rs"), b"fn main() {}").expect("test setup");
        mark(&mut app, "project");

        copy_with(&mut app, &typed(purpose, "release"));

        let release = from.path().join("release");
        assert_eq!(
            fs::read(release.join("src/main.rs")).expect("the contents"),
            b"fn main() {}",
            "{purpose:?}"
        );
        assert!(!release.join("project").exists(), "{purpose:?} nested it");
    }
}

/// An existing folder is somewhere to put things, whether it was typed or
/// offered.
#[test]
fn an_existing_folder_receives_the_item_under_its_own_name() {
    let (mut app, from, _to) = app_with(&[("notes.txt", b"x")]);
    fs::create_dir(from.path().join("archive")).expect("test setup");
    mark(&mut app, "notes.txt");

    copy_with(&mut app, &typed(Purpose::Copy, "archive"));

    assert!(from.path().join("archive/notes.txt").is_file());
}

/// "If you want to create the destination folder before copying, append
/// backslash to its name" — Far's help; a slash here.
#[test]
fn a_trailing_separator_creates_the_folder_and_puts_the_item_in_it() {
    let (mut app, from, _to) = app_with(&[("notes.txt", b"x")]);
    mark(&mut app, "notes.txt");

    let folder = format!("archive{}", std::path::MAIN_SEPARATOR);
    copy_with(&mut app, &typed(Purpose::Copy, &folder));

    assert!(from.path().join("archive/notes.txt").is_file());
}

/// Several items cannot all take one name, so a new name for them is a
/// folder to put them in.
#[test]
fn several_items_sent_to_a_new_name_go_into_a_folder_of_that_name() {
    let (mut app, from, _to) = app_with(&[("a.txt", b"a"), ("b.txt", b"b")]);
    mark(&mut app, "a.txt");
    mark(&mut app, "b.txt");

    copy_with(&mut app, &typed(Purpose::Copy, "bundle"));

    assert_eq!(fs::read(from.path().join("bundle/a.txt")).expect("a"), b"a");
    assert_eq!(fs::read(from.path().join("bundle/b.txt")).expect("b"), b"b");
}

/// A new name that turns out to be taken by a file goes through the "already
/// existing files" choice, instead of failing to create a folder there.
#[test]
fn a_name_taken_by_a_file_follows_the_existing_files_choice() {
    for (existing, kept, beside) in [
        (ExistingFiles::Ask, &b"old"[..], Some(&b"new"[..])),
        (ExistingFiles::Overwrite, &b"new"[..], None),
        (ExistingFiles::Skip, &b"old"[..], None),
    ] {
        let (mut app, from, _to) = app_with(&[("draft.txt", b"new"), ("final.txt", b"old")]);
        mark(&mut app, "draft.txt");

        let mut dialog = typed(Purpose::Copy, "final.txt");
        dialog.existing = existing;
        copy_with(&mut app, &dialog);

        let read = |name: &str| fs::read(from.path().join(name)).ok();
        assert_eq!(read("final.txt").as_deref(), Some(kept), "{existing:?}");
        assert_eq!(read("final_1.txt").as_deref(), beside, "{existing:?}");
    }
}

/// A new name inside the folder being copied is still inside it, and is
/// refused before anything is created there.
#[test]
fn a_new_name_inside_the_folder_itself_is_refused() {
    let (mut app, from, _to) = app_with(&[]);
    fs::create_dir(from.path().join("project")).expect("test setup");
    mark(&mut app, "project");

    copy_with(&mut app, &typed(Purpose::Copy, "project/backup"));

    assert!(!from.path().join("project/backup").exists());
    assert!(
        app.status.to_string().contains("into itself"),
        "{}",
        app.status
    );
}
