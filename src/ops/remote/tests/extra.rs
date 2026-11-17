//! Purpose: Extra unit tests for remote selection, parent-row permissions, and local symlink transfer guards.

use tempfile::TempDir;

use super::super::{actions, completion, transfer};
use crate::action::{Action, FileCommand};
use crate::models::remote_work::{
    FileLocation, RemoteOutcome, RemoteRequest, RemoteTarget, RemoteTask,
};
use crate::models::{EntryKind, PanelKind, PanelSide, SessionsPanel};
use crate::remote::{RemoteError, UnixMode};
use crate::status::Status;
use crate::test_support::netbox::{app_on_host, session};

#[test]
fn parent_permissions_and_unselect_group_are_handled_on_remote_panel() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app_on_host(dir.path(), &[("/etc/hosts", 4)], Some("/etc"));
    assert!(actions::handle(
        &mut app,
        Action::File(FileCommand::Attributes)
    ));
    assert!(!app.is_overlay_active());
    assert!(actions::handle(&mut app, Action::UnselectGroup));
    assert!(app.is_overlay_active());
}

#[test]
fn local_symlink_metadata_chmod_and_transfer_are_guarded() {
    let dir = TempDir::new().expect("temp dir");
    let real = dir.path().join("real");
    std::fs::create_dir(&real).expect("mkdir");
    let link = dir.path().join("link");
    crate::test_support::make_dir_link(&real, &link).expect("link");
    let loc = FileLocation::Local(link);
    assert_eq!(
        transfer::metadata(&loc).expect("meta").kind,
        EntryKind::SymlinkToFile
    );
    assert!(matches!(
        transfer::chmod(&loc, UnixMode::from_mode(0o644)),
        Err(RemoteError::Unsupported(_))
    ));
    assert_eq!(
        transfer::transfer(
            &loc,
            &FileLocation::Local(dir.path().join("copied_link")),
            false
        ),
        Err(RemoteError::Unsupported(
            "copying symbolic links between hosts"
        ))
    );
}

#[test]
fn connect_target_and_refresh_with_deleted_local_dir_or_quickview_succeed() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app_on_host(dir.path(), &[("/file", 3)], None);
    let PanelKind::Remote(panel) = &app.left_panel.kind else {
        panic!("remote panel")
    };
    let (link, path) = (panel.link().clone(), panel.path().clone());
    let name = session("example.test", Some("demo")).name;
    let mut list = SessionsPanel::from_store(&app.sessions);
    list.focus(&name);
    app.right_panel.kind = PanelKind::Sessions(Box::new(list));
    let req = RemoteRequest {
        id: 2,
        target: RemoteTarget::Connect {
            side: PanelSide::Right,
            session: name,
        },
        task: RemoteTask::List {
            link: link.clone(),
            path: path.clone(),
        },
    };
    let listed = RemoteOutcome::Listed {
        link,
        path,
        entries: Vec::new(),
    };
    completion::apply(&mut app, req, Ok(listed));
    assert!(matches!(app.right_panel.kind, PanelKind::Remote(_)));
}

#[test]
fn refresh_reports_error_on_deleted_local_dir_and_skips_quick_view() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app_on_host(dir.path(), &[("/file", 3)], None);
    app.left_panel.path = std::path::PathBuf::from("\0");
    app.left_panel.kind = PanelKind::Files;
    app.right_panel.kind = PanelKind::QuickView(Box::default());
    completion::refresh(&mut app);
    assert!(matches!(app.status, Status::Error(_)));
}
