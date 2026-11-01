//! Local and remote transfers retain their safety rules for moves and errors.
use super::*;

#[test]
fn move_between_two_hosts_copies_the_complete_tree_before_removing_source() {
    let source_root = remote();
    let target_root = remote();
    let temp = tempfile::TempDir::new().expect("temporary directory");
    let local = temp.path().join("tree");
    fs::create_dir_all(local.join("nested")).expect("directory");
    fs::write(local.join("nested/file"), b"payload").expect("file");
    let source = source_root.child("tree").expect("remote source");
    transfer(&FileLocation::Local(local), &source, false).expect("seed");
    let target = target_root.child("copy").expect("target");
    transfer(&source, &target, true).expect("cross-host move");
    assert!(metadata(&source).is_err());
    let downloaded = FileLocation::Local(temp.path().join("downloaded"));
    transfer(&target, &downloaded, false).expect("download");
    assert_eq!(
        fs::read(temp.path().join("downloaded/nested/file")).expect("file"),
        b"payload"
    );
}

#[test]
fn same_host_move_renames_tree_and_rejects_a_destination_inside_itself() {
    let host = remote();
    let source = host.child("source").expect("source");
    mkdir(&source).expect("directory");
    let nested = source.child("nested").expect("child");
    assert!(matches!(
        transfer(&source, &nested, true),
        Err(RemoteError::InvalidInput(_))
    ));
    assert!(metadata(&source).is_ok());
    let target = host.child("renamed").expect("target");
    transfer(&source, &target, true).expect("rename");
    assert!(metadata(&source).is_err());
    assert!(metadata(&target).expect("destination").kind.is_dir());
}

#[test]
fn local_copy_into_self_is_rejected_and_local_tree_can_be_removed() {
    let temp = tempfile::TempDir::new().expect("temporary directory");
    let source = temp.path().join("source");
    fs::create_dir_all(source.join("nested")).expect("directory");
    fs::write(source.join("nested/file"), b"content").expect("file");
    let location = FileLocation::Local(source.clone());
    let inside = FileLocation::Local(source.join("nested/copy"));
    assert!(matches!(
        transfer(&location, &inside, false),
        Err(RemoteError::InvalidInput(_))
    ));
    assert!(source.join("nested/file").exists());
    remove(&location).expect("remove tree");
    assert!(!source.exists());
}

#[cfg(unix)]
#[test]
fn symbolic_link_copy_and_permission_change_are_refused() {
    use std::os::unix::fs::symlink;
    let temp = tempfile::TempDir::new().expect("temporary directory");
    let actual = temp.path().join("actual");
    let link = temp.path().join("link");
    fs::write(&actual, b"original").expect("file");
    symlink(&actual, &link).expect("symlink");
    let source = FileLocation::Local(link);
    assert!(matches!(
        transfer(&source, &remote().child("copy").expect("target"), false),
        Err(RemoteError::Unsupported(_))
    ));
    assert!(matches!(
        chmod(&source, crate::remote::UnixMode::from_mode(0o600)),
        Err(RemoteError::Unsupported(_))
    ));
    remove(&source).expect("remove link");
    assert_eq!(fs::read(actual).expect("original remains"), b"original");
}
