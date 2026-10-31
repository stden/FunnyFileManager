//! Purpose: Placement edge cases: names with no file name, a case variant that
//! the folder still lists, a destination the folder cannot be inspected through,
//! and a keep-both copy whose source has gone.

use super::placement::is_case_only_rename;
use super::*;
use crate::copy::{CopyPrompt, ExistingFiles};
use tempfile::TempDir;

#[test]
fn a_root_path_has_no_name_to_rename_from() {
    assert!(!is_case_only_rename(Path::new("/"), Path::new("/data")));
}

#[test]
fn case_variants_in_different_folders_are_not_a_rename() {
    let first = TempDir::new().unwrap();
    let second = TempDir::new().unwrap();
    fs::write(first.path().join("data.txt"), b"a").unwrap();
    fs::write(second.path().join("DATA.txt"), b"b").unwrap();

    assert!(!is_case_only_rename(
        &first.path().join("data.txt"),
        &second.path().join("DATA.txt"),
    ));
}

#[cfg(target_os = "linux")]
#[test]
fn a_case_variant_link_to_the_source_is_not_a_rename_of_it() {
    let root = TempDir::new().unwrap();
    let source = root.path().join("data.txt");
    fs::write(&source, b"payload").unwrap();
    std::os::unix::fs::symlink(&source, root.path().join("DATA.txt")).unwrap();

    // The folder lists "DATA.txt" as an entry of its own, so the name is taken.
    assert!(!is_case_only_rename(&source, &root.path().join("DATA.txt")));
    assert_eq!(fs::read(&source).unwrap(), b"payload");
}

#[test]
fn a_destination_under_a_file_cannot_be_inspected() {
    let root = TempDir::new().unwrap();
    let source = root.path().join("source.txt");
    fs::write(&source, b"payload").unwrap();
    let blocker = root.path().join("blocker");
    fs::write(&blocker, b"not a folder").unwrap();
    let dest = blocker.join("child");
    let prompt = CopyPrompt::new("source", root.path());

    let result = place_over_existing(&source, &dest, &prompt);

    assert!(result.is_err());
    assert_eq!(fs::read(&blocker).unwrap(), b"not a folder");
    assert_eq!(fs::read(&source).unwrap(), b"payload");
}

#[test]
fn keep_both_reports_a_source_that_vanished_and_creates_nothing_beside_the_original() {
    let root = TempDir::new().unwrap();
    let dest = root.path().join("data");
    fs::write(&dest, b"original").unwrap();
    let mut prompt = CopyPrompt::new("data", root.path());
    prompt.existing = ExistingFiles::Ask;

    let result = place_over_existing(&root.path().join("gone"), &dest, &prompt);

    assert!(result.is_err());
    assert_eq!(fs::read(&dest).unwrap(), b"original");
    assert_eq!(
        fs::read_dir(root.path()).unwrap().count(),
        1,
        "no copy was left beside the original"
    );
}

#[cfg(target_os = "linux")]
#[test]
fn a_case_variant_that_is_not_there_is_not_a_rename_of_the_source() {
    let root = TempDir::new().unwrap();
    let source = root.path().join("data.txt");
    fs::write(&source, b"payload").unwrap();

    assert!(!is_case_only_rename(&source, &root.path().join("DATA.txt")));
}
