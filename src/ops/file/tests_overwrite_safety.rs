//! Unit tests: what "Overwrite" may and may not destroy.
//!
//! Overwriting replaces a file the transfer brings with it. It never removes
//! anything the transfer does not bring: a folder copied onto a folder of the
//! same name is merged into it, as Far, mc and Explorer do; a file and a folder
//! never replace one another; and a folder is never overwritten by something
//! it holds, because removing it would take the source with it.

use super::tests_transfer_policies::{app_between, mark, settings};
use super::*;

use crate::copy::{ExistingFiles, Purpose};

use tempfile::TempDir;

/// `from/docs/{a.txt, new.txt}` onto `to/docs/{a.txt, only_here.txt}`.
fn two_docs_folders() -> (App, TempDir, TempDir) {
    let from = TempDir::new().expect("test setup");
    let to = TempDir::new().expect("test setup");
    let source = from.path().join("docs");
    fs::create_dir(&source).expect("test setup");
    fs::write(source.join("a.txt"), b"new a").expect("test setup");
    fs::write(source.join("new.txt"), b"brand new").expect("test setup");
    let existing = to.path().join("docs");
    fs::create_dir(&existing).expect("test setup");
    fs::write(existing.join("a.txt"), b"old a").expect("test setup");
    fs::write(existing.join("only_here.txt"), b"precious").expect("test setup");
    let mut app = app_between(from.path(), to.path());
    mark(&mut app, "docs");
    (app, from, to)
}

fn overwrite(to: &Path, purpose: Purpose) -> crate::copy::CopyPrompt {
    let mut dialog = settings(to);
    dialog.existing = ExistingFiles::Overwrite;
    dialog.purpose = purpose;
    dialog
}

#[test]
fn copying_a_folder_onto_one_merges_and_keeps_what_only_the_destination_has() {
    let (mut app, _from, to) = two_docs_folders();

    copy_with(&mut app, &overwrite(to.path(), Purpose::Copy));

    let docs = to.path().join("docs");
    assert_eq!(fs::read(docs.join("only_here.txt")).unwrap(), b"precious");
    assert_eq!(fs::read(docs.join("a.txt")).unwrap(), b"new a");
    assert_eq!(fs::read(docs.join("new.txt")).unwrap(), b"brand new");
}

#[test]
fn moving_a_folder_onto_one_merges_and_takes_the_source_away() {
    let (mut app, from, to) = two_docs_folders();

    copy_with(&mut app, &overwrite(to.path(), Purpose::Move));

    let docs = to.path().join("docs");
    assert_eq!(fs::read(docs.join("only_here.txt")).unwrap(), b"precious");
    assert_eq!(fs::read(docs.join("a.txt")).unwrap(), b"new a");
    assert_eq!(fs::read(docs.join("new.txt")).unwrap(), b"brand new");
    assert!(!from.path().join("docs").exists(), "{}", app.status);
}

#[test]
fn only_newer_decides_file_by_file_inside_a_merged_folder() {
    let (mut app, _from, to) = two_docs_folders();
    let docs = to.path().join("docs");
    // The destination's a.txt is newer than the source's: it stays.
    let later = std::time::SystemTime::now() + std::time::Duration::from_secs(3600);
    fs::File::options()
        .write(true)
        .open(docs.join("a.txt"))
        .and_then(|file| file.set_modified(later))
        .expect("test setup");
    let mut dialog = settings(to.path());
    dialog.existing = ExistingFiles::OnlyNewer;

    copy_with(&mut app, &dialog);

    assert_eq!(fs::read(docs.join("a.txt")).unwrap(), b"old a");
    assert_eq!(fs::read(docs.join("new.txt")).unwrap(), b"brand new");
    assert_eq!(fs::read(docs.join("only_here.txt")).unwrap(), b"precious");
}

#[test]
fn a_file_never_overwrites_a_folder() {
    let from = TempDir::new().expect("test setup");
    let to = TempDir::new().expect("test setup");
    fs::write(from.path().join("docs"), b"a file").expect("test setup");
    fs::create_dir(to.path().join("docs")).expect("test setup");
    fs::write(to.path().join("docs/precious.txt"), b"precious").expect("test setup");
    let mut app = app_between(from.path(), to.path());
    mark(&mut app, "docs");

    copy_with(&mut app, &overwrite(to.path(), Purpose::Copy));

    assert_eq!(
        fs::read(to.path().join("docs/precious.txt")).unwrap(),
        b"precious"
    );
    assert!(
        app.status.to_string().contains("1 error"),
        "the refusal is reported: {}",
        app.status
    );
}

#[test]
fn a_folder_never_overwrites_a_file() {
    let from = TempDir::new().expect("test setup");
    let to = TempDir::new().expect("test setup");
    fs::create_dir(from.path().join("docs")).expect("test setup");
    fs::write(to.path().join("docs"), b"precious").expect("test setup");
    let mut app = app_between(from.path(), to.path());
    mark(&mut app, "docs");

    copy_with(&mut app, &overwrite(to.path(), Purpose::Move));

    assert_eq!(fs::read(to.path().join("docs")).unwrap(), b"precious");
    assert!(from.path().join("docs").is_dir(), "the source stays put");
    assert!(
        app.status.to_string().contains("1 error"),
        "the refusal is reported: {}",
        app.status
    );
}

/// `root/pkg/pkg` sent to `root`, where the `pkg` already there is the folder
/// that holds the source.
fn nested_same_name() -> (App, TempDir) {
    let root = TempDir::new().expect("test setup");
    let outer = root.path().join("pkg");
    let inner = outer.join("pkg");
    fs::create_dir_all(&inner).expect("test setup");
    fs::write(inner.join("data.txt"), b"payload").expect("test setup");
    fs::write(outer.join("other.txt"), b"other").expect("test setup");
    let mut app = app_between(&outer, root.path());
    mark(&mut app, "pkg");
    (app, root)
}

#[test]
fn copying_a_folder_onto_the_folder_that_holds_it_is_refused() {
    let (mut app, root) = nested_same_name();

    copy_with(&mut app, &overwrite(root.path(), Purpose::Copy));

    let outer = root.path().join("pkg");
    assert_eq!(fs::read(outer.join("pkg/data.txt")).unwrap(), b"payload");
    assert_eq!(fs::read(outer.join("other.txt")).unwrap(), b"other");
    assert!(
        app.status.to_string().contains("1 error"),
        "the refusal is reported: {}",
        app.status
    );
}

#[test]
fn moving_a_folder_onto_the_folder_that_holds_it_is_refused() {
    let (mut app, root) = nested_same_name();

    copy_with(&mut app, &overwrite(root.path(), Purpose::Move));

    let outer = root.path().join("pkg");
    assert_eq!(fs::read(outer.join("pkg/data.txt")).unwrap(), b"payload");
    assert_eq!(fs::read(outer.join("other.txt")).unwrap(), b"other");
    assert!(
        app.status.to_string().contains("1 error"),
        "the refusal is reported: {}",
        app.status
    );
}
