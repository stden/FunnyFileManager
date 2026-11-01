//! Locations validate names, and queued work cannot be overwritten or completed by a stale reply.

use super::{FileLocation, RemoteRequests, RemoteTarget, RemoteTask};
use crate::models::{PanelSide, RemotePath};
use crate::remote::fake::FakeRemoteFs;
use crate::remote::{RemoteError, RemoteLink};
use std::path::PathBuf;

fn remote(text: &str) -> FileLocation {
    FileLocation::Remote {
        link: RemoteLink::new(Box::new(FakeRemoteFs::default())),
        path: RemotePath::parse(text).expect("absolute remote path"),
    }
}

#[test]
fn remote_names_must_be_single_path_components() {
    let base = remote("/work");
    let child = base.child("report.txt").expect("valid name");
    assert_eq!(child.label(), "/work/report.txt");
    assert_eq!(child.name(), Ok("report.txt"));
    assert_eq!(child.parent(), Ok(base.clone()));
    for invalid in ["", ".", "..", "a/b"] {
        assert!(base.child(invalid).is_err(), "invalid name: {invalid}");
    }
    assert!(remote("/").parent().is_err());
    assert!(remote("/").name().is_err());
}

#[test]
fn local_names_must_be_single_path_components() {
    let base = FileLocation::Local(PathBuf::from("work"));
    let child = base.child("report.txt").expect("valid name");
    assert_eq!(child.name(), Ok("report.txt"));
    assert_eq!(child.parent(), Ok(base.clone()));
    for invalid in ["", ".", "..", "a/b"] {
        assert!(base.child(invalid).is_err(), "invalid name: {invalid}");
    }
}

#[test]
fn remote_destinations_resolve_relative_or_absolute_paths_and_reject_invalid_input() {
    let base = remote("/work");
    assert_eq!(
        base.resolve("sub/file").expect("relative").label(),
        "/work/sub/file"
    );
    assert_eq!(
        base.resolve("/elsewhere").expect("absolute").label(),
        "/elsewhere"
    );
    assert!(base.resolve("").is_err());
    assert!(base.resolve("../../escape").is_err());
}

#[test]
fn local_destinations_resolve_from_the_directory_or_keep_an_absolute_path() {
    let temp = tempfile::tempdir().expect("temporary directory");
    let base = FileLocation::Local(temp.path().to_path_buf());
    let child = base.resolve("sub/file").expect("relative");
    assert_eq!(child, FileLocation::Local(temp.path().join("sub/file")));
    let absolute = temp.path().join("absolute");
    assert_eq!(
        base.resolve(absolute.to_str().expect("utf8")),
        Ok(FileLocation::Local(absolute))
    );
    assert!(base.resolve("").is_err());
}

#[test]
fn request_queue_rejects_overlaps_and_ignores_stale_completion() {
    let link = RemoteLink::new(Box::new(FakeRemoteFs::default()));
    let path = RemotePath::root();
    let target = RemoteTarget::Panel {
        side: PanelSide::Left,
        link: link.clone(),
        path: path.clone(),
    };
    let task = RemoteTask::List { link, path };
    let mut requests = RemoteRequests::default();
    assert!(requests.is_idle());
    assert!(requests.take().is_none());
    requests
        .queue(target.clone(), task.clone())
        .expect("first request");
    assert_eq!(
        requests.queue(target.clone(), task.clone()),
        Err(RemoteError::Failed("a remote operation is already running"))
    );
    let first = requests.take().expect("queued request");
    assert!(!requests.finish(first.id + 1));
    assert!(requests.take().is_none());
    assert!(requests.finish(first.id));
    assert!(requests.is_idle());
    requests.queue(target, task).expect("next request");
    let next = requests.take().expect("second request");
    assert_ne!(first.id, next.id);
    assert!(!requests.finish(first.id));
    assert!(requests.finish(next.id));
}
