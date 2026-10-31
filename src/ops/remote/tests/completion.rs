//! Remote refresh results must belong to the visible panel before being applied.

use super::super::completion;
use crate::models::remote_work::{
    RemoteListing, RemoteOutcome, RemoteRefresh, RemoteRequest, RemoteTarget, RemoteTask,
};
use crate::models::{App, DirEntry, EntryKind, PanelKind, PanelSide, RemotePath};
use crate::remote::{RemoteError, RemoteLink};
use crate::status::Status;

fn host() -> (tempfile::TempDir, App, RemoteLink, RemotePath) {
    let dir = tempfile::TempDir::new().expect("temporary directory");
    let app = crate::test_support::netbox::app_on_host(dir.path(), &[("/file", 3)], None);
    let PanelKind::Remote(panel) = &app.left_panel.kind else {
        panic!("remote panel")
    };
    let link = panel.link().clone();
    let path = panel.path().clone();
    (dir, app, link, path)
}

fn listing(link: &RemoteLink, path: RemotePath) -> RemoteListing {
    RemoteListing {
        side: PanelSide::Left,
        link: link.clone(),
        path,
    }
}

fn request(link: &RemoteLink) -> RemoteRequest {
    let path = RemotePath::root();
    RemoteRequest {
        id: 1,
        target: RemoteTarget::Panel {
            side: PanelSide::Left,
            link: link.clone(),
            path: path.clone(),
        },
        task: RemoteTask::List {
            link: link.clone(),
            path,
        },
    }
}

#[test]
fn failed_refresh_keeps_the_listing_and_reports_the_error() {
    let (_dir, mut app, link, path) = host();
    let refresh = RemoteRefresh {
        listing: listing(&link, path.clone()),
        result: Err(RemoteError::NotFound(path)),
    };
    completion::apply(
        &mut app,
        request(&link),
        Ok(RemoteOutcome::Refreshed(vec![refresh])),
    );
    assert!(matches!(app.status, Status::Error(_)));
    let PanelKind::Remote(panel) = &app.left_panel.kind else {
        panic!("remote panel")
    };
    assert!(panel.entries().iter().any(|entry| entry.name == "file"));
}

#[test]
fn refresh_for_a_previous_directory_is_ignored() {
    let (_dir, mut app, link, _) = host();
    let entry = DirEntry {
        name: "intruder".into(),
        kind: EntryKind::File,
        ..DirEntry::default()
    };
    let other = RemotePath::parse("/elsewhere").expect("path");
    let refresh = RemoteRefresh {
        listing: listing(&link, other),
        result: Ok(vec![entry]),
    };
    completion::apply(
        &mut app,
        request(&link),
        Ok(RemoteOutcome::Refreshed(vec![refresh])),
    );
    let PanelKind::Remote(panel) = &app.left_panel.kind else {
        panic!("remote panel")
    };
    assert!(!panel.entries().iter().any(|entry| entry.name == "intruder"));
}

#[test]
fn refresh_for_a_closed_panel_is_ignored() {
    let (_dir, mut app, link, _) = host();
    app.left_panel.kind = PanelKind::Files;
    let refresh = RemoteRefresh {
        listing: listing(&link, RemotePath::root()),
        result: Ok(Vec::new()),
    };
    completion::apply(
        &mut app,
        request(&link),
        Ok(RemoteOutcome::Refreshed(vec![refresh])),
    );
    assert!(matches!(app.left_panel.kind, PanelKind::Files));
}

#[test]
fn invalid_remote_names_are_reported_without_showing_them() {
    let (_dir, mut app, link, path) = host();
    let entry = DirEntry {
        name: "../escape".into(),
        kind: EntryKind::File,
        ..DirEntry::default()
    };
    let refresh = RemoteRefresh {
        listing: listing(&link, path),
        result: Ok(vec![entry]),
    };
    completion::apply(
        &mut app,
        request(&link),
        Ok(RemoteOutcome::Refreshed(vec![refresh])),
    );
    assert!(app.status_text().contains("cannot be represented"));
    let PanelKind::Remote(panel) = &app.left_panel.kind else {
        panic!("remote panel")
    };
    assert!(!panel
        .entries()
        .iter()
        .any(|entry| entry.name.contains("escape")));
}

#[test]
fn successful_mutation_queues_a_refresh_for_open_remote_panels() {
    let (_dir, mut app, link, _) = host();
    completion::apply(&mut app, request(&link), Ok(RemoteOutcome::Done));
    let request = app.remote_requests.take().expect("refresh queued");
    let RemoteTask::Refresh(listings) = request.task else {
        panic!("refresh")
    };
    assert_eq!(listings.len(), 1);
    assert_eq!(listings[0].path, RemotePath::root());
}
