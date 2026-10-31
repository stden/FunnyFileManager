//! Purpose: Merging a folder into an existing one while following links: the
//! contents of the folder land in the existing one, and a link that leads back
//! into the copy or into a folder being merged is refused rather than followed.

use super::merge::{merge_into, reject_merge_into_holder};
use super::*;
use crate::copy::{CopyPrompt, ExistingFiles};
use tempfile::TempDir;

/// A copy that follows links and replaces what is already there.
fn following(root: &Path) -> CopyPrompt {
    let mut prompt = CopyPrompt::new("source", root);
    prompt.existing = ExistingFiles::Overwrite;
    prompt.symlink_contents = true;
    prompt
}

#[test]
fn a_folder_merged_into_an_existing_one_lands_its_contents_there() {
    let root = TempDir::new().unwrap();
    let source = root.path().join("source");
    let dest = root.path().join("dest");
    fs::create_dir_all(source.join("sub")).unwrap();
    fs::write(source.join("sub").join("new.txt"), b"new").unwrap();
    fs::create_dir_all(dest.join("sub")).unwrap();

    let skipped = merge_into(&source, &dest, &following(root.path())).expect("the merge succeeds");

    assert_eq!(skipped, 0);
    assert_eq!(fs::read(dest.join("sub").join("new.txt")).unwrap(), b"new");
}

#[test]
fn a_merge_whose_paths_do_not_resolve_is_left_to_fail_later() {
    let root = TempDir::new().unwrap();
    let dest = root.path().join("dest");
    fs::create_dir(&dest).unwrap();

    let result = reject_merge_into_holder(&root.path().join("missing"), &dest);

    assert!(result.is_ok());
}

#[cfg(unix)]
#[test]
fn a_link_in_the_source_that_leads_into_the_copy_is_refused() {
    let root = TempDir::new().unwrap();
    let source = root.path().join("source");
    let dest = root.path().join("dest");
    fs::create_dir(&source).unwrap();
    fs::create_dir(&dest).unwrap();
    std::os::unix::fs::symlink(&dest, source.join("into-copy")).unwrap();

    let error = merge_into(&source, &dest, &following(root.path())).expect_err("the link loops");

    assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
    assert!(
        error.to_string().contains("leads into the copy itself"),
        "{error}"
    );
}

#[cfg(unix)]
#[test]
fn a_link_back_to_a_folder_being_merged_is_a_loop() {
    let root = TempDir::new().unwrap();
    let source = root.path().join("source");
    let dest = root.path().join("dest");
    fs::create_dir(&source).unwrap();
    fs::create_dir(&dest).unwrap();
    std::os::unix::fs::symlink(&source, source.join("up")).unwrap();

    let error = merge_into(&source, &dest, &following(root.path())).expect_err("the link loops");

    assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
    assert!(error.to_string().contains("symbolic link loop"), "{error}");
}
