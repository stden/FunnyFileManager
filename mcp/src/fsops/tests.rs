//! Unit tests for the guarded filesystem operations.

use super::{apply_readonly, remove_entry, write_replaced};
use std::fs;
use std::path::Path;
use tempfile::TempDir;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

#[cfg(unix)]
fn mode_of(path: &std::path::Path) -> u32 {
    fs::metadata(path).expect("metadata").permissions().mode() & 0o7777
}

/// Sorted names directly inside `dir` — used to assert no temp file survives.
fn entry_names(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .expect("read_dir")
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

#[cfg(unix)]
#[test]
fn deleting_a_link_to_a_directory_keeps_the_target() {
    let dir = TempDir::new().expect("temp dir");
    let tree = dir.path().join("data/tree");
    fs::create_dir_all(&tree).expect("mkdir");
    fs::write(tree.join("keep.txt"), "keep").expect("write");

    let link = dir.path().join("link_to_tree");
    std::os::unix::fs::symlink(&tree, &link).expect("symlink");

    remove_entry(&link, true).expect("removing the link must succeed");

    assert!(!link.is_symlink(), "the link itself should be gone");
    assert!(
        tree.join("keep.txt").exists(),
        "the target tree must survive"
    );
}

#[cfg(unix)]
#[test]
fn deleting_a_dangling_link_succeeds() {
    let dir = TempDir::new().expect("temp dir");
    let link = dir.path().join("dangling");
    std::os::unix::fs::symlink(dir.path().join("no_such_target"), &link).expect("symlink");

    remove_entry(&link, true).expect("a dangling link must still be removable");
    assert!(!link.is_symlink());
}

#[test]
fn removes_files_and_directories() {
    let dir = TempDir::new().expect("temp dir");
    let file = dir.path().join("f.txt");
    fs::write(&file, "x").expect("write");
    remove_entry(&file, false).expect("remove file");
    assert!(!file.exists());

    let nested = dir.path().join("d/inner");
    fs::create_dir_all(&nested).expect("mkdir");
    remove_entry(&dir.path().join("d"), false).expect_err("non-recursive remove of a full dir");
    remove_entry(&dir.path().join("d"), true).expect("recursive remove");
    assert!(!nested.exists());
}

#[test]
fn missing_path_is_reported_not_ignored() {
    let dir = TempDir::new().expect("temp dir");
    remove_entry(&dir.path().join("ghost"), true).expect_err("missing path must error");
}

#[test]
fn write_replaced_swaps_content() {
    let dir = TempDir::new().expect("temp dir");
    let file = dir.path().join("a.txt");
    fs::write(&file, "old").expect("write");

    write_replaced(&file, "new").expect("replace");
    assert_eq!(fs::read_to_string(&file).expect("read"), "new");

    assert_eq!(
        entry_names(dir.path()),
        vec!["a.txt".to_owned()],
        "no temp file left behind"
    );
}

#[cfg(unix)]
#[test]
fn write_replaced_preserves_permissions() {
    let dir = TempDir::new().expect("temp dir");
    let file = dir.path().join("secret.txt");
    fs::write(&file, "old").expect("write");
    fs::set_permissions(&file, fs::Permissions::from_mode(0o600)).expect("chmod");

    write_replaced(&file, "new").expect("replace");
    assert_eq!(mode_of(&file), 0o600, "mode must survive the rename");
}

#[test]
fn write_replaced_reports_failure_and_keeps_the_original() {
    let dir = TempDir::new().expect("temp dir");
    // A directory is not a regular file, so the replacement is refused for
    // every user — root included, which ignores mode bits — before anything is
    // written or renamed.
    let target = dir.path().join("occupied");
    fs::create_dir(&target).expect("mkdir");
    fs::write(target.join("inside.txt"), "keep").expect("write");

    let error = write_replaced(&target, "new").expect_err("the failure must be surfaced");
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    assert_eq!(
        fs::read_to_string(target.join("inside.txt")).expect("read"),
        "keep",
        "a failed replacement must leave the target untouched"
    );
    assert_eq!(
        entry_names(dir.path()),
        vec!["occupied".to_owned()],
        "a failed replacement must not leave a temp file behind"
    );
}

#[test]
fn write_replaced_refuses_a_target_that_does_not_exist() {
    let dir = TempDir::new().expect("temp dir");

    write_replaced(&dir.path().join("ghost.txt"), "new").expect_err("there is nothing to replace");
    assert!(
        entry_names(dir.path()).is_empty(),
        "a refused replacement must not create anything"
    );
}

/// Regression: the temp file used to be the fully predictable sibling
/// `.<name>.ffm-tmp`, opened with `fs::write` — no `O_EXCL`, no `O_NOFOLLOW`.
/// Planting that name as a symlink made the server write our content through
/// it, chmod the *link target* to the replaced file's mode, and then rename the
/// link over the real file.
#[cfg(unix)]
#[test]
fn write_replaced_cannot_be_hijacked_by_a_planted_temp_file() {
    let dir = TempDir::new().expect("temp dir");
    let outside = TempDir::new().expect("temp dir");

    let victim = outside.path().join("authorized_keys");
    fs::write(&victim, "SECRET").expect("write");
    fs::set_permissions(&victim, fs::Permissions::from_mode(0o644)).expect("chmod");

    let target = dir.path().join("settings.json");
    fs::write(&target, "old").expect("write");
    fs::set_permissions(&target, fs::Permissions::from_mode(0o600)).expect("chmod");

    let planted = dir.path().join(".settings.json.ffm-tmp");
    std::os::unix::fs::symlink(&victim, &planted).expect("symlink");

    write_replaced(&target, "new").expect("the replacement itself must still work");

    assert_eq!(fs::read_to_string(&target).expect("read"), "new");
    assert_eq!(mode_of(&target), 0o600, "mode must survive the rename");
    assert_eq!(
        fs::read_to_string(&victim).expect("read"),
        "SECRET",
        "the planted link must not have been written through"
    );
    assert_eq!(
        mode_of(&victim),
        0o644,
        "the planted link's target must not have been chmod'ed"
    );
    // `symlink_metadata`, never `exists()`: `exists()` follows the link and
    // would still report true after the link had been renamed away.
    assert!(
        fs::symlink_metadata(&planted)
            .expect("the planted link is still there")
            .file_type()
            .is_symlink(),
        "the planted link must not have been renamed over the target"
    );
}

/// Regression: renaming over a symlink silently turns it into a regular file,
/// and the content that was read came from wherever the link pointed.
#[cfg(unix)]
#[test]
fn write_replaced_refuses_to_replace_a_symlink() {
    let dir = TempDir::new().expect("temp dir");
    let real = dir.path().join("real.txt");
    fs::write(&real, "old").expect("write");
    let link = dir.path().join("link.txt");
    std::os::unix::fs::symlink(&real, &link).expect("symlink");

    write_replaced(&link, "new").expect_err("a symlink is not a regular file");

    assert_eq!(fs::read_to_string(&real).expect("read"), "old");
    assert!(
        fs::symlink_metadata(&link)
            .expect("the link is still there")
            .file_type()
            .is_symlink(),
        "the link must not have been turned into a regular file"
    );
}

#[test]
fn write_replaced_rejects_a_path_without_a_file_name() {
    // Without this guard `with_file_name` would turn "/" into "/.ffm-tmp" and
    // the replacement would write outside the intended file entirely.
    let error =
        write_replaced(Path::new("/"), "x").expect_err("'/' has no name to build a temp file from");

    assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    assert!(
        error.to_string().contains("path has no file name"),
        "the caller needs to know why, got: {error}"
    );
    assert!(
        !Path::new("/.ffm-tmp").exists(),
        "no temp file may be created outside the target directory"
    );
}

#[cfg(unix)]
#[test]
fn clearing_readonly_does_not_widen_to_group_or_world() {
    let dir = TempDir::new().expect("temp dir");
    let file = dir.path().join("key");
    fs::write(&file, "private").expect("write");
    fs::set_permissions(&file, fs::Permissions::from_mode(0o600)).expect("chmod");

    apply_readonly(&file, false).expect("clear readonly");
    assert_eq!(
        mode_of(&file),
        0o600,
        "owner write stays, group/world must not gain it"
    );
}

#[cfg(unix)]
#[test]
fn setting_readonly_clears_every_write_bit() {
    let dir = TempDir::new().expect("temp dir");
    let file = dir.path().join("shared");
    fs::write(&file, "data").expect("write");
    fs::set_permissions(&file, fs::Permissions::from_mode(0o666)).expect("chmod");

    apply_readonly(&file, true).expect("set readonly");
    assert_eq!(mode_of(&file), 0o444);

    apply_readonly(&file, false).expect("clear readonly");
    assert_eq!(mode_of(&file), 0o644, "only the owner bit comes back");
}
