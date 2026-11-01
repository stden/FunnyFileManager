//! Remote jobs use a fake host to test real reads, writes, and refreshes.

use super::super::jobs;
use crate::models::remote_work::{FileLocation, RemoteListing, RemoteOutcome, RemoteTask};
use crate::models::{AfterSave, Fingerprint, PanelSide, RemotePath};
use crate::remote::{fake::FakeRemoteFs, RemoteLink, UnixMode, WriteMode};

fn path(text: &str) -> RemotePath {
    RemotePath::parse(text).expect("remote path")
}

fn location(link: &RemoteLink, text: &str) -> FileLocation {
    FileLocation::Remote {
        link: link.clone(),
        path: path(text),
    }
}

fn host_with_file() -> RemoteLink {
    let link = RemoteLink::new(Box::new(FakeRemoteFs::default()));
    link.lock()
        .upload(
            &path("/file"),
            &mut b"original".as_slice(),
            WriteMode::CreateNew,
        )
        .expect("seed");
    link
}

#[test]
fn listing_and_opening_report_real_contents() {
    let link = host_with_file();
    let root = path("/");
    let outcome = jobs::execute(&RemoteTask::List {
        link: link.clone(),
        path: root,
    })
    .expect("list");
    let RemoteOutcome::Listed { entries, .. } = outcome else {
        panic!("listing")
    };
    assert_eq!(entries[0].name, "file");
    let outcome = jobs::execute(&RemoteTask::Open {
        source: location(&link, "/file"),
        edit: true,
    })
    .expect("open");
    let RemoteOutcome::Opened { bytes, edit, .. } = outcome else {
        panic!("opened file")
    };
    assert!(edit);
    assert_eq!(bytes, b"original");
}

#[test]
fn refresh_returns_success_and_failure_for_each_listing() {
    let link = host_with_file();
    let listing = |path| RemoteListing {
        side: PanelSide::Left,
        link: link.clone(),
        path,
    };
    let outcome = jobs::execute(&RemoteTask::Refresh(vec![
        listing(path("/")),
        listing(path("/missing")),
    ]))
    .expect("refresh");
    let RemoteOutcome::Refreshed(refreshed) = outcome else {
        panic!("refresh")
    };
    assert!(refreshed[0].result.is_ok());
    assert!(refreshed[1].result.is_err());
}

#[test]
fn save_detects_conflicts_and_overwrite_publishes_the_new_content() {
    let link = host_with_file();
    let make = |expected, overwrite| RemoteTask::Save {
        source: location(&link, "/file"),
        bytes: b"updated".to_vec(),
        expected,
        overwrite,
        then: AfterSave::Stay,
        revision: 0,
    };
    assert!(jobs::execute(&make(Fingerprint::of(b"outdated"), false)).is_err());
    assert!(matches!(
        jobs::execute(&make(Fingerprint::of(b"outdated"), true)),
        Ok(RemoteOutcome::Saved)
    ));
    let mut content = Vec::new();
    link.lock()
        .download(&path("/file"), &mut content)
        .expect("read");
    assert_eq!(content, b"updated");
}

fn renamed_file(link: &RemoteLink) -> FileLocation {
    let directory = location(link, "/directory");
    assert!(matches!(
        jobs::execute(&RemoteTask::Mkdir(directory)),
        Ok(RemoteOutcome::Done)
    ));
    let renamed = location(link, "/directory/renamed");
    assert!(matches!(
        jobs::execute(&RemoteTask::Rename {
            from: location(link, "/file"),
            to: renamed.clone()
        }),
        Ok(RemoteOutcome::Done)
    ));
    renamed
}

#[test]
fn mkdir_rename_permissions_and_delete_execute_on_the_host() {
    let link = host_with_file();
    let renamed = renamed_file(&link);
    let mode = UnixMode::from_mode(0o600);
    assert!(matches!(
        jobs::execute(&RemoteTask::Chmod {
            targets: vec![renamed],
            mode
        }),
        Ok(RemoteOutcome::Done)
    ));
    assert_eq!(
        link.lock()
            .stat(&path("/directory/renamed"))
            .expect("stat")
            .mode,
        Some(mode)
    );
    assert!(matches!(
        jobs::execute(&RemoteTask::Delete(vec![location(&link, "/directory")])),
        Ok(RemoteOutcome::Done)
    ));
    assert!(link.lock().stat(&path("/directory/renamed")).is_err());
}

#[test]
fn open_enforces_size_limit_even_when_a_host_reports_more_data() {
    let link = RemoteLink::new(Box::new(FakeRemoteFs::with_files(&[(
        "/large",
        10 * 1024 * 1024 + 1,
    )])));
    let task = RemoteTask::Open {
        source: location(&link, "/large"),
        edit: false,
    };
    assert!(jobs::execute(&task).is_err());
}

#[test]
fn local_open_reads_data_without_connecting_to_a_host() {
    let temp = tempfile::TempDir::new().expect("temporary directory");
    let source = temp.path().join("note.txt");
    std::fs::write(&source, b"local").expect("file");
    let task = RemoteTask::Open {
        source: FileLocation::Local(source),
        edit: false,
    };
    let RemoteOutcome::Opened { bytes, edit, .. } = jobs::execute(&task).expect("open") else {
        panic!("opened file")
    };
    assert!(!edit);
    assert_eq!(bytes, b"local");
}
