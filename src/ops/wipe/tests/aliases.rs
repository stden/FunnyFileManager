//! Purpose: The last checks on the open file keep a hard link's other names
//! intact, and clearing read-only never follows a link or minds a gone entry.

use super::*;

/// The link count is read again through the open handle: a file that has
/// another name is refused before a single zero is written.
#[test]
fn a_hard_link_found_by_the_open_handle_is_never_zeroed() {
    let root = TempDir::new().expect("temp dir");
    let source = root.path().join("source");
    let alias = root.path().join("alias");
    fs::write(&source, b"keep").expect("write");
    fs::hard_link(&source, &alias).expect("hard link");

    let error = super::super::steps::zero_fill(&source).expect_err("refused");

    assert!(error.to_string().contains("multiple hard links"));
    assert_eq!(fs::read(source).expect("source"), b"keep");
    assert_eq!(fs::read(alias).expect("alias"), b"keep");
}

/// Clearing read-only on a link leaves its target's mode alone, and a missing
/// entry is no error.
#[test]
fn clearing_a_missing_entry_or_a_symlink_leaves_the_target_mode_unchanged() {
    use std::os::unix::fs::PermissionsExt;

    let root = TempDir::new().expect("temp dir");
    let source = root.path().join("source");
    let link = root.path().join("link");
    fs::write(&source, b"keep").expect("write");
    fs::set_permissions(&source, fs::Permissions::from_mode(0o400)).expect("chmod 0400");
    std::os::unix::fs::symlink(&source, &link).expect("symlink");

    super::super::clear_readonly(&link);
    super::super::clear_readonly(&root.path().join("missing"));

    assert_eq!(
        fs::metadata(source).expect("meta").permissions().mode() & 0o777,
        0o400
    );
}
