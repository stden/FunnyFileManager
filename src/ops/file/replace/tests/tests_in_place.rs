//! Purpose: an overwrite keeps what the replaced file is — its inode, owner,
//! mode and other names — and a refusal met before writing keeps its data.

use std::os::unix::fs::{MetadataExt, PermissionsExt};

use super::*;
use crate::copy::ExistingFiles;

/// `source` holding "new", `dest` holding "old", and the dialog set to
/// overwrite in `root`.
fn overwrite() -> (TempDir, std::path::PathBuf, std::path::PathBuf, CopyPrompt) {
    let root = TempDir::new().unwrap();
    let source = root.path().join("source");
    let dest = root.path().join("dest");
    fs::write(&source, b"new").unwrap();
    fs::write(&dest, b"old").unwrap();
    let mut prompt = CopyPrompt::new("source", root.path());
    prompt.existing = ExistingFiles::Overwrite;
    (root, source, dest, prompt)
}

#[test]
fn a_hard_linked_file_is_written_through_so_every_name_sees_it() {
    let (root, source, dest, prompt) = overwrite();
    let other = root.path().join("other-name");
    fs::hard_link(&dest, &other).unwrap();
    let inode = fs::metadata(&dest).unwrap().ino();

    assert_eq!(replacement(&source, &dest, &prompt), Replacement::InPlace);
    entry(&source, &dest, &prompt).unwrap();

    assert_eq!(fs::read(&other).unwrap(), b"new", "the other name follows");
    assert_eq!(fs::metadata(&dest).unwrap().ino(), inode);
    assert_eq!(fs::metadata(&dest).unwrap().nlink(), 2);
}

#[test]
fn overwriting_a_hard_link_through_a_symlink_to_same_inode_preserves_contents() {
    let (root, _source, dest, mut prompt) = overwrite();
    let hard = root.path().join("hard");
    fs::hard_link(&dest, &hard).unwrap();
    let link = root.path().join("link");
    std::os::unix::fs::symlink(&dest, &link).unwrap();
    prompt.symlink_contents = true;

    assert!(entry(&link, &hard, &prompt).is_err());
    assert_eq!(
        fs::read(&dest).unwrap(),
        b"old",
        "must not truncate same inode to 0 bytes"
    );
    assert_eq!(fs::read(&hard).unwrap(), b"old");
}

/// Root hands the file to someone else; other users cannot, and skip.
#[test]
fn another_users_writable_file_keeps_its_owner_and_mode() {
    const SOMEONE_ELSE: u32 = 4_242_424;
    let (_root, source, dest, prompt) = overwrite();
    if std::os::unix::fs::chown(&dest, Some(SOMEONE_ELSE), Some(SOMEONE_ELSE)).is_err() {
        return;
    }
    fs::set_permissions(&dest, fs::Permissions::from_mode(0o666)).unwrap();
    let inode = fs::metadata(&dest).unwrap().ino();

    entry(&source, &dest, &prompt).unwrap();

    let meta = fs::metadata(&dest).unwrap();
    assert_eq!((meta.uid(), meta.gid()), (SOMEONE_ELSE, SOMEONE_ELSE));
    assert_eq!(meta.permissions().mode() & 0o7777, 0o666);
    assert_eq!(meta.ino(), inode);
    assert_eq!(fs::read(&dest).unwrap(), b"new");
}

/// The file may be written though its folder takes no new entry: no temporary
/// sibling can be made there, so the file is written through.
#[test]
fn a_writable_file_in_a_read_only_folder_is_overwritten() {
    let (root, source, _, prompt) = overwrite();
    let locked = root.path().join("locked");
    fs::create_dir(&locked).unwrap();
    let dest = locked.join("file.txt");
    fs::write(&dest, b"old").unwrap();
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o555)).unwrap();

    let result = entry(&source, &dest, &prompt);
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).unwrap();

    result.unwrap();
    assert_eq!(fs::read(&dest).unwrap(), b"new");
    assert_eq!(fs::read_dir(&locked).unwrap().count(), 1, "no staging left");
}

#[test]
fn an_overwrite_keeps_the_replaced_mode_unless_rights_are_copied() {
    for (rights, expected) in [(AccessRights::Default, 0o600), (AccessRights::Copy, 0o644)] {
        let (_root, source, dest, mut prompt) = overwrite();
        fs::set_permissions(&source, fs::Permissions::from_mode(0o644)).unwrap();
        fs::set_permissions(&dest, fs::Permissions::from_mode(0o600)).unwrap();
        prompt.rights = rights;

        entry(&source, &dest, &prompt).unwrap();

        let mode = fs::metadata(&dest).unwrap().permissions().mode() & 0o7777;
        assert_eq!(mode, expected, "{rights:?}");
    }
}

/// A move replaces the name, as `mv` does: the file's other names keep the
/// old contents, exactly as after a rename on one filesystem.
#[test]
fn a_move_replaces_the_name_rather_than_writing_through() {
    let (root, source, dest, mut prompt) = overwrite();
    fs::hard_link(&dest, root.path().join("other-name")).unwrap();
    prompt.purpose = Purpose::Move;

    assert_eq!(replacement(&source, &dest, &prompt), Replacement::Staged);
}

/// A time the file will not take is found out while its old contents are
/// whole, and the status names the file that refused rather than the source.
#[test]
fn a_refusal_before_writing_keeps_the_old_contents_and_names_the_file() {
    use crate::models::{App, FileName, Panel};
    use crate::utils::FORCE_TIMES_FAILURE;

    let (root, _source, dest, mut prompt) = overwrite();
    fs::hard_link(&dest, root.path().join("other-name")).unwrap();
    let panel = || Panel::new(root.path().to_path_buf()).unwrap();
    let mut app = App::with_panels(panel(), panel());
    app.get_active_panel()
        .marked
        .insert(FileName::new("source"));
    prompt.destination.set_text(&dest.display().to_string());
    prompt.timestamps = true;

    FORCE_TIMES_FAILURE.set(true);
    crate::ops::file::copy_with(&mut app, &prompt);
    FORCE_TIMES_FAILURE.set(false);

    assert_eq!(fs::read(&dest).unwrap(), b"old");
    let status = app.status.to_string();
    assert!(status.ends_with("dest: forced times failure"), "{status}");
}
