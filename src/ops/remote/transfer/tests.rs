//! Purpose: Local/remote transfers preserve data and never delete a source after failure.

use super::*;
use crate::models::RemotePath;
use crate::remote::{fake::FakeRemoteFs, RemoteLink};

mod more;

fn remote() -> FileLocation {
    FileLocation::Remote {
        link: RemoteLink::new(Box::new(FakeRemoteFs::default())),
        path: RemotePath::root(),
    }
}

#[test]
fn nested_directories_round_trip_through_a_remote_panel() {
    let temp = tempfile::TempDir::new().expect("temporary directory");
    let source = temp.path().join("source");
    fs::create_dir_all(source.join("nested")).expect("mkdir");
    fs::write(source.join("nested/file.txt"), b"remote contents").expect("file");
    let host = remote();
    let uploaded = host.child("source").expect("remote destination");
    transfer(&FileLocation::Local(source), &uploaded, false).expect("upload directory");
    let target = FileLocation::Local(temp.path().join("downloaded"));
    transfer(&uploaded, &target, false).expect("download directory");
    assert_eq!(
        fs::read(temp.path().join("downloaded/nested/file.txt")).expect("download"),
        b"remote contents"
    );
    remove(&uploaded).expect("remote recursive delete");
    assert!(metadata(&uploaded).is_err());
}

#[test]
fn a_failed_move_keeps_the_source_and_existing_destination() {
    let temp = tempfile::TempDir::new().expect("temporary directory");
    let source = FileLocation::Local(temp.path().join("source"));
    fs::write(temp.path().join("source"), b"original").expect("source");
    let destination = remote().child("file").expect("destination");
    transfer(&source, &destination, false).expect("first copy");
    assert!(transfer(&source, &destination, true).is_err());
    assert_eq!(
        fs::read(temp.path().join("source")).expect("source survives"),
        b"original"
    );
    let FileLocation::Remote { link, path } = destination else {
        panic!("remote");
    };
    let mut bytes = Vec::new();
    link.lock()
        .download(&path, &mut bytes)
        .expect("destination survives");
    assert_eq!(bytes, b"original");
}
