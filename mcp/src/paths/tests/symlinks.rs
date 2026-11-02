//! Purpose: Pins that no symlink — final, intermediate, dangling, or on the
//! way to the root itself — lets a path resolve outside the root.

use super::{root_of, RootEnvGuard};
use crate::paths::{
    resolve_root, sanitize_path, sanitize_path_create, sanitize_path_nofollow, ROOT_ENV,
};
use std::fs;
use tempfile::TempDir;

#[cfg(unix)]
#[test]
fn rejects_symlink_pointing_out_of_root() {
    let dir = TempDir::new().expect("temp dir");
    let outside = TempDir::new().expect("temp dir");
    let root = root_of(&dir);
    let target = root_of(&outside).join("secret.txt");
    fs::write(&target, "secret").expect("write file");

    let link = root.join("escape");
    std::os::unix::fs::symlink(&target, &link).expect("symlink");

    let error = sanitize_path(&root, &link.display().to_string())
        .expect_err("a symlink out of the root must be rejected");
    assert!(error.contains("outside the allowed root"), "got: {error}");

    // The link itself lives inside the root, so a no-follow resolution is fine:
    // that is what lets `delete` remove the link without touching the target.
    let resolved = sanitize_path_nofollow(&root, &link.display().to_string())
        .expect("the link path itself is inside the root");
    assert_eq!(resolved, link);
}

#[cfg(unix)]
#[test]
fn create_rejects_a_new_path_below_a_symlink_out_of_root() {
    let dir = TempDir::new().expect("temp dir");
    let outside = TempDir::new().expect("temp dir");
    let root = root_of(&dir);
    let outside_root = root_of(&outside);
    std::os::unix::fs::symlink(&outside_root, root.join("data")).expect("symlink");

    let read_error =
        sanitize_path(&root, "data/x.txt").expect_err("reads through the link must be refused");
    assert!(
        read_error.contains("outside the allowed root"),
        "{read_error}"
    );

    let error = sanitize_path_create(&root, "data/newdir/x.txt")
        .expect_err("a new path below a symlink out of the root must be refused");
    assert!(error.contains("outside the allowed root"), "got: {error}");
    assert!(
        fs::symlink_metadata(outside_root.join("newdir")).is_err(),
        "nothing may be created outside the root"
    );
}

#[cfg(unix)]
#[test]
fn create_refuses_to_write_through_a_dangling_symlink() {
    let dir = TempDir::new().expect("temp dir");
    let outside = TempDir::new().expect("temp dir");
    let root = root_of(&dir);
    let target = root_of(&outside).join("victim.txt");
    let link = root.join("link");
    std::os::unix::fs::symlink(&target, &link).expect("symlink");

    let error = sanitize_path_create(&root, "link")
        .expect_err("a dangling link must not be written through");
    assert!(error.contains("dangling symlink"), "got: {error}");

    assert!(
        fs::symlink_metadata(&link)
            .expect("the link itself is still there")
            .file_type()
            .is_symlink(),
        "the link must be left untouched"
    );
    assert!(
        fs::symlink_metadata(&target).is_err(),
        "the link target must not have been created outside the root"
    );
}

#[cfg(unix)]
#[test]
fn create_accepts_new_parents_when_the_root_is_reached_through_a_symlink() {
    let dir = TempDir::new().expect("temp dir");
    let real = dir.path().join("real");
    fs::create_dir_all(real.join("proj")).expect("mkdir");
    let via_link = dir.path().join("home");
    std::os::unix::fs::symlink(&real, &via_link).expect("symlink");

    let _guard = RootEnvGuard::acquire();
    std::env::set_var(ROOT_ENV, via_link.join("proj"));
    let root = resolve_root().expect("the symlinked root resolves");
    fs::write(root.join("existing.txt"), "hi").expect("write file");

    let existing = via_link.join("proj/existing.txt").display().to_string();
    assert_eq!(
        sanitize_path_create(&root, &existing).expect("an existing file was always accepted"),
        root.join("existing.txt")
    );

    let fresh = via_link.join("proj/deep/new/f.txt").display().to_string();
    assert_eq!(
        sanitize_path_create(&root, &fresh)
            .expect("a path under the symlinked root is inside the root"),
        root.join("deep/new/f.txt"),
        "both branches must compare canonical to canonical"
    );
}

#[cfg(unix)]
#[test]
fn nofollow_rejects_escape_through_the_parent() {
    let dir = TempDir::new().expect("temp dir");
    let outside = TempDir::new().expect("temp dir");
    let root = root_of(&dir);

    let link = root.join("out");
    std::os::unix::fs::symlink(root_of(&outside), &link).expect("symlink");

    let error = sanitize_path_nofollow(&root, &link.join("file.txt").display().to_string())
        .expect_err("the parent must be confined too");
    assert!(error.contains("outside the allowed root"), "got: {error}");
}

#[cfg(unix)]
#[test]
fn create_resolves_in_root_symlinks_before_folding_parent_dir_components() {
    let dir = TempDir::new().expect("temp dir");
    let root = root_of(&dir);
    fs::create_dir_all(root.join("a/b/c")).expect("mkdir a/b/c");
    std::os::unix::fs::symlink(&root, root.join("a/b/c/up")).expect("symlink up -> root");

    // `a/b/c/up` resolves to `root`, so `../../../missing_outside` escapes `root`
    // even when the target does not exist on disk yet.
    let err = sanitize_path_create(&root, "a/b/c/up/../../../missing_outside")
        .expect_err("symlink to root followed by '..' must not fold lexically");
    assert!(err.contains("outside the allowed root"), "got: {err}");

    // And an in-root symlink to a deeper directory resolves `..` relative to the
    // link's target directory, not the link's parent directory.
    fs::create_dir_all(root.join("other/deep")).expect("mkdir other/deep");
    fs::create_dir_all(root.join("sub")).expect("mkdir sub");
    std::os::unix::fs::symlink(root.join("other/deep"), root.join("sub/link"))
        .expect("symlink sub/link -> other/deep");
    let resolved = sanitize_path_create(&root, "sub/link/../fresh.txt").expect("inside root");
    assert_eq!(resolved, root.join("other/fresh.txt"));
}
