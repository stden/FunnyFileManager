//! Unit tests: the fake host lists what it was given, directories above a
//! file included, and fails the way a host fails.

use super::FakeRemoteFs;
use crate::models::{EntryKind, RemotePath};
use crate::remote::{RemoteError, RemoteFs};

fn path(text: &str) -> RemotePath {
    RemotePath::parse(text).expect("path")
}

#[test]
fn a_directory_lists_its_files_and_the_directories_under_it() {
    let mut fs = FakeRemoteFs::with_files(&[("/var/www/index.html", 12), ("/etc/hostname", 5)]);
    let root = fs.list(&RemotePath::root()).expect("root");
    let names: Vec<&str> = root.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(names, ["etc", "var"]);
    assert!(root.iter().all(|e| e.kind == EntryKind::Directory));

    let www = fs.list(&path("/var/www")).expect("www");
    assert_eq!(www.len(), 1);
    assert_eq!(www[0].name, "index.html");
    assert_eq!(www[0].kind, EntryKind::File);
    assert_eq!(www[0].size.bytes(), 12);
}

#[test]
fn a_missing_path_or_a_file_cannot_be_listed() {
    let mut fs = FakeRemoteFs::with_files(&[("/etc/hostname", 5)]);
    assert_eq!(
        fs.list(&path("/nope")),
        Err(RemoteError::NotFound(path("/nope")))
    );
    assert_eq!(
        fs.list(&path("/etc/hostname")),
        Err(RemoteError::NotADirectory(path("/etc/hostname")))
    );
}

#[test]
fn a_path_that_does_not_parse_is_left_out() {
    let mut fs = FakeRemoteFs::with_files(&[("relative", 1), ("/ok", 2)]);
    let root = fs.list(&RemotePath::root()).expect("root");
    let names: Vec<&str> = root.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(names, ["ok"]);
    assert_eq!(
        FakeRemoteFs::default().list(&RemotePath::root()),
        Ok(Vec::new())
    );
}
