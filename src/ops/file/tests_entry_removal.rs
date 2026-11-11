//! Purpose: A removal the kernel refuses is reported at the step that refused
//! it, names the entry inside a folder, and leaves what was not refused in
//! place. A copy into a folder that cannot be created names the destination.

use std::time::Duration;

use tempfile::TempDir;

use super::removal::{remove_path, RemovalStep, FORCE_REMOVAL_FAILURE};
use super::*;
use crate::progress::{install_with, Operation, Painter, Snapshot};
use crate::utils::{CopyRules, FailedAt};

/// Forces one removal step to fail until dropped.
pub(super) struct ForcedRemoval;

impl ForcedRemoval {
    pub(super) fn step(step: RemovalStep) -> Self {
        FORCE_REMOVAL_FAILURE.set(Some(step));
        Self
    }
}

impl Drop for ForcedRemoval {
    fn drop(&mut self) {
        FORCE_REMOVAL_FAILURE.set(None);
    }
}

/// Draws nothing; installing it makes a removal count as shown in progress.
struct Silent;

impl Painter for Silent {
    fn paint(&mut self, _snapshot: &Snapshot) {}
}

/// `root/folder` with `a.txt` and `sub/b.txt`, removed as a shown delete.
fn shown_tree(root: &Path) -> PathBuf {
    let folder = root.join("folder");
    fs::create_dir_all(folder.join("sub")).unwrap();
    fs::write(folder.join("a.txt"), b"abc").unwrap();
    fs::write(folder.join("sub").join("b.txt"), b"hello").unwrap();
    folder
}

#[test]
fn a_refused_file_removal_is_reported_and_the_file_stays() {
    let root = TempDir::new().unwrap();
    let file = root.path().join("data.txt");
    fs::write(&file, b"payload").unwrap();

    let _forced = ForcedRemoval::step(RemovalStep::RemoveFile);
    let error = remove_path(&file).expect_err("the removal is refused");

    assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
    assert_eq!(fs::read(&file).unwrap(), b"payload");
}

#[test]
fn a_refused_entry_inside_a_shown_folder_is_named() {
    let root = TempDir::new().unwrap();
    let folder = shown_tree(root.path());
    let _installed = install_with(Box::new(Silent), Duration::ZERO);
    let _run = crate::progress::begin(Operation::Delete, std::slice::from_ref(&folder));

    let _forced = ForcedRemoval::step(RemovalStep::RemoveFile);
    let error = remove_path(&folder).expect_err("the first file is refused");

    let named = FailedAt::of(&error).expect("the refusal names its entry");
    assert!(
        named.path().starts_with(&folder),
        "{}",
        named.path().display()
    );
    assert!(named.path().is_file(), "a refused file is still there");
}

#[test]
fn a_folder_that_cannot_be_listed_is_reported_and_kept() {
    let root = TempDir::new().unwrap();
    let folder = shown_tree(root.path());
    let _installed = install_with(Box::new(Silent), Duration::ZERO);
    let _run = crate::progress::begin(Operation::Delete, std::slice::from_ref(&folder));

    let _forced = ForcedRemoval::step(RemovalStep::ListFolder);
    let error = remove_path(&folder).expect_err("the listing is refused");

    assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
    assert!(folder.join("a.txt").exists());
}

#[test]
fn a_listing_that_breaks_partway_stops_at_the_folder() {
    let root = TempDir::new().unwrap();
    let folder = shown_tree(root.path());
    let _installed = install_with(Box::new(Silent), Duration::ZERO);
    let _run = crate::progress::begin(Operation::Delete, std::slice::from_ref(&folder));

    let _forced = ForcedRemoval::step(RemovalStep::NextEntry);
    let error = remove_path(&folder).expect_err("the listing breaks");

    assert_eq!(FailedAt::of(&error).unwrap().path(), folder.as_path());
    assert!(folder.exists());
}

#[test]
fn an_entry_whose_kind_cannot_be_read_is_named_and_kept() {
    let root = TempDir::new().unwrap();
    let folder = shown_tree(root.path());
    let _installed = install_with(Box::new(Silent), Duration::ZERO);
    let _run = crate::progress::begin(Operation::Delete, std::slice::from_ref(&folder));

    let _forced = ForcedRemoval::step(RemovalStep::EntryKind);
    let error = remove_path(&folder).expect_err("the kind is refused");

    assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
    assert!(folder.join("a.txt").exists());
}

#[test]
fn a_nested_folder_that_cannot_be_removed_stops_the_whole_removal() {
    let root = TempDir::new().unwrap();
    let folder = shown_tree(root.path());
    let _installed = install_with(Box::new(Silent), Duration::ZERO);
    let _run = crate::progress::begin(Operation::Delete, std::slice::from_ref(&folder));

    let _forced = ForcedRemoval::step(RemovalStep::RemoveDir);
    let error = remove_path(&folder).expect_err("the subfolder is refused");

    assert_eq!(
        FailedAt::of(&error).unwrap().path(),
        folder.join("sub").as_path()
    );
    assert!(folder.join("sub").exists());
}

#[test]
fn a_folder_that_cannot_be_removed_once_empty_is_reported() {
    let root = TempDir::new().unwrap();
    let folder = root.path().join("empty");
    fs::create_dir(&folder).unwrap();
    let _installed = install_with(Box::new(Silent), Duration::ZERO);
    let _run = crate::progress::begin(Operation::Delete, std::slice::from_ref(&folder));

    let _forced = ForcedRemoval::step(RemovalStep::RemoveDir);
    let error = remove_path(&folder).expect_err("the empty folder is refused");

    assert_eq!(FailedAt::of(&error).unwrap().path(), folder.as_path());
    assert!(folder.exists());
}

#[test]
fn a_folder_copied_into_a_missing_parent_names_the_destination() {
    let root = TempDir::new().unwrap();
    let source = shown_tree(root.path());
    let dest = root.path().join("no-such-parent").join("copy");

    let error = copy_entry(&source, &dest, CopyRules::PLAIN).expect_err("no parent exists");

    assert_eq!(error.kind(), io::ErrorKind::NotFound);
    assert_eq!(FailedAt::of(&error).unwrap().path(), dest.as_path());
    assert!(!dest.exists());
}
