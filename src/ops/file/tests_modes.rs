//! Unit tests: the permission bits a copy ends up with, for each choice of
//! the dialog's "Access rights".
//!
//! Default is what `fs::copy` gives a file: the source's bits, so a script
//! stays executable and a private file stays private. Copy restates the
//! source's bits on a folder as well.

use std::os::unix::fs::PermissionsExt;

use super::tests_transfer_policies::{app_with, mark, settings};
use super::*;
use crate::copy::{AccessRights, Purpose};

/// The permission bits of `path` in octal, as `ls` and `chmod` spell them,
/// so a failing comparison reads `644` rather than `420`.
fn mode(path: &Path) -> String {
    let bits = fs::metadata(path).expect("metadata").permissions().mode();
    format!("{:o}", bits & 0o7777)
}

fn octal(mode: u32) -> String {
    format!("{mode:o}")
}

fn set_mode(path: &Path, mode: u32) {
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).expect("test setup");
}

#[test]
fn copy_rights_carries_the_source_mode_over() {
    let (mut app, from, to) = app_with(&[("run.sh", b"#!/bin/sh\n")]);
    set_mode(&from.path().join("run.sh"), 0o755);
    mark(&mut app, "run.sh");

    let mut dialog = settings(to.path());
    dialog.rights = AccessRights::Copy;
    copy_with(&mut app, &dialog);

    assert_eq!(mode(&to.path().join("run.sh")), octal(0o755));
}

/// A script keeps its executable bit, and a private file does not pick up
/// the read bits a freshly created file would get — the same as a file copied
/// inside a folder, and the same as `cp`.
#[test]
fn default_rights_keep_the_mode_of_a_copied_file() {
    for source_mode in [0o755, 0o600] {
        let (mut app, from, to) = app_with(&[("run.sh", b"#!/bin/sh\n")]);
        set_mode(&from.path().join("run.sh"), source_mode);
        mark(&mut app, "run.sh");

        copy_with(&mut app, &settings(to.path()));

        assert_eq!(
            mode(&to.path().join("run.sh")),
            octal(source_mode),
            "the copy of a {source_mode:o} file"
        );
    }
}

/// The quick copy and the F6 fallback across filesystems go through the same
/// file copy, so they keep the mode as well.
#[test]
fn the_quick_copy_and_the_move_fallback_keep_the_mode() {
    let _guard = RENAME_TEST_MUTEX
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let (mut app, from, to) = app_with(&[("run.sh", b"#!/bin/sh\n")]);
    set_mode(&from.path().join("run.sh"), 0o755);
    mark(&mut app, "run.sh");
    handle_copy(&mut app).expect("refresh");
    assert_eq!(
        mode(&to.path().join("run.sh")),
        octal(0o755),
        "the quick copy"
    );

    let elsewhere = tempfile::TempDir::new().expect("test setup");
    mark(&mut app, "run.sh");
    let mut dialog = settings(elsewhere.path());
    dialog.purpose = Purpose::Move;
    FORCE_RENAME_FAILURE.store(true, Ordering::SeqCst);
    copy_with(&mut app, &dialog);
    FORCE_RENAME_FAILURE.store(false, Ordering::SeqCst);
    assert_eq!(
        mode(&elsewhere.path().join("run.sh")),
        octal(0o755),
        "the move that had to copy"
    );
}

/// What a file and a folder newly created in `dir` get. Only the kernel
/// knows — it applies the umask, or the folder's default ACL, at creation —
/// so the one honest way to ask is to create one of each.
fn fresh_modes(dir: &Path) -> (String, String) {
    let (file, folder) = (dir.join("probe.txt"), dir.join("probe.d"));
    fs::File::create(&file).expect("probe");
    fs::create_dir(&folder).expect("probe");
    let modes = (mode(&file), mode(&folder));
    fs::remove_file(&file).expect("probe");
    fs::remove_dir(&folder).expect("probe");
    modes
}

/// Inherit is what a newly created file in the destination gets, which is
/// the point: a script or a private file copied into a shared directory
/// stops carrying its own mode. Setting "a fresh mode" afterwards with
/// `chmod` skips the umask — that made every such copy world-writable.
#[test]
fn inherit_gives_a_copied_file_the_mode_a_new_file_there_gets() {
    for source_mode in [0o755, 0o600] {
        let (mut app, from, to) = app_with(&[("run.sh", b"#!/bin/sh\n")]);
        set_mode(&from.path().join("run.sh"), source_mode);
        mark(&mut app, "run.sh");

        let mut dialog = settings(to.path());
        dialog.rights = AccessRights::Inherit;
        copy_with(&mut app, &dialog);

        let (fresh_file, _) = fresh_modes(to.path());
        assert_eq!(
            mode(&to.path().join("run.sh")),
            fresh_file,
            "the copy of a {source_mode:o} file"
        );
    }
}

/// A folder is created afresh, and so is everything inside it: Far applies
/// the choice to every file and folder the copy makes, not only the top one.
#[test]
fn inherit_gives_a_copied_folder_and_all_it_holds_fresh_modes() {
    let (mut app, from, to) = app_with(&[]);
    let project = from.path().join("project");
    fs::create_dir_all(project.join("bin")).expect("test setup");
    fs::write(project.join("bin/run.sh"), b"#!/bin/sh\n").expect("test setup");
    set_mode(&project.join("bin/run.sh"), 0o755);
    set_mode(&project.join("bin"), 0o700);
    set_mode(&project, 0o700);
    mark(&mut app, "project");

    let mut dialog = settings(to.path());
    dialog.rights = AccessRights::Inherit;
    copy_with(&mut app, &dialog);

    let (fresh_file, fresh_folder) = fresh_modes(to.path());
    let copy = to.path().join("project");
    assert_eq!(mode(&copy), fresh_folder, "the folder itself");
    assert_eq!(mode(&copy.join("bin")), fresh_folder, "a folder inside");
    assert_eq!(mode(&copy.join("bin/run.sh")), fresh_file, "a file inside");
}
