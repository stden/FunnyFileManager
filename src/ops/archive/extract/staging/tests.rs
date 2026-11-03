//! Purpose: The staging folder is checked for links and clashing destination links before anything moves.

use std::fs;

use tempfile::TempDir;

use super::{existing, plain_tree, settle};

#[test]
fn a_link_unpacked_into_the_staging_folder_is_refused() {
    let temp = TempDir::new().expect("temp");
    let target = temp.path().join("real");
    fs::create_dir_all(&target).expect("target");
    let staged = TempDir::new().expect("staged");
    let docs = staged.path().join("docs");
    fs::create_dir_all(&docs).expect("docs");
    fs::write(docs.join("a.txt"), "a").expect("file");
    crate::test_support::make_dir_link(&target, &docs.join("etc")).expect("link");

    let error = plain_tree(staged.path()).expect_err("a link");
    assert!(error.to_string().contains("etc"), "{error}");
}

#[test]
fn settle_merges_staged_directory_and_refuses_destination_link_clash() {
    let staging = TempDir::new().expect("staging");
    let dest = TempDir::new().expect("dest");
    let staged_sub = staging.path().join("sub");
    fs::create_dir_all(&staged_sub).expect("staged sub");
    fs::write(staged_sub.join("file.txt"), "hello").expect("file");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&staged_sub, fs::Permissions::from_mode(0o555)).expect("chmod");
    }
    let dest_sub = dest.path().join("sub");
    fs::create_dir_all(&dest_sub).expect("dest sub");
    settle(staging.path(), dest.path()).expect("settle merges folders");
    assert_eq!(
        fs::read_to_string(dest_sub.join("file.txt")).expect("read"),
        "hello"
    );

    let staging2 = TempDir::new().expect("staging2");
    fs::write(staging2.path().join("linked"), "x").expect("staged file");
    let dest2 = TempDir::new().expect("dest2");
    crate::test_support::make_dir_link(dest.path(), &dest2.path().join("linked")).expect("link");
    let err = settle(staging2.path(), dest2.path()).expect_err("link clash");
    assert!(err.to_string().contains("is a link"), "{err}");
    assert!(existing(std::path::Path::new("bad\0name")).is_err());
}
