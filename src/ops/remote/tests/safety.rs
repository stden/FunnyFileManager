use super::super::{actions, context, editor, enqueue};
use crate::command::language::{Action, FileCommand};
use crate::models::remote_work::{FileLocation, RemoteTarget, RemoteTask};
use crate::models::{Fingerprint, PanelSide, RemotePath};
use crate::remote::RemoteError;
use crate::test_support::netbox::{app_in, app_on_host, app_on_sessions, file_store, session};
use tempfile::TempDir;

#[test]
fn session_lists_cannot_be_used_as_transfer_directories_or_files() {
    let dir = TempDir::new().expect("temporary directory");
    let app = app_on_sessions(
        dir.path(),
        file_store(dir.path(), vec![session("fixture.invalid", None)]),
    );
    assert!(matches!(
        context::directory(&app.left_panel),
        Err(RemoteError::Unsupported(_))
    ));
    assert!(matches!(
        context::sources(&app.left_panel),
        Err(RemoteError::Unsupported(_))
    ));
    assert!(matches!(
        context::selected(&app.left_panel),
        Err(RemoteError::Unsupported(_))
    ));
}

#[test]
fn a_local_editor_source_is_rejected_by_remote_save() {
    let dir = TempDir::new().expect("temporary directory");
    let path = dir.path().join("local.txt");
    std::fs::write(&path, b"original").expect("local file");
    assert_eq!(
        editor::save(
            &FileLocation::Local(path.clone()),
            Fingerprint::of(b"original"),
            b"replacement",
            true
        ),
        Err(RemoteError::Failed("no remote editor source"))
    );
    assert_eq!(
        std::fs::read(path).expect("local source stays"),
        b"original"
    );
}

#[test]
fn a_second_remote_request_reports_busy_without_replacing_the_queued_one() {
    let dir = TempDir::new().expect("temporary directory");
    let mut app = app_on_host(dir.path(), &[], None);
    let target = context::operation_target(&app).expect("remote target");
    enqueue(&mut app, target.clone(), RemoteTask::Refresh(Vec::new()));
    enqueue(&mut app, target, RemoteTask::Delete(Vec::new()));
    assert!(app.status_text().contains("already running"));
    assert!(matches!(
        app.remote_requests.take().expect("first request").task,
        RemoteTask::Refresh(_)
    ));
}

#[test]
fn transfer_to_an_unsupported_panel_reports_the_error() {
    let dir = TempDir::new().expect("temporary directory");
    let mut app = app_on_host(dir.path(), &[("/file", 1)], None);
    let sessions = app_on_sessions(dir.path(), file_store(dir.path(), Vec::new()));
    app.right_panel.kind = sessions.left_panel.kind;
    if let crate::models::PanelKind::Remote(panel) = &mut app.left_panel.kind {
        panel.move_cursor(crate::models::RowMotion::Next);
    }
    assert!(actions::handle(&mut app, Action::Copy));
    assert!(app.status_text().contains("transfer to this panel kind"));
}

#[test]
fn an_unrelated_action_and_a_local_operation_do_not_enter_remote_workflows() {
    let dir = TempDir::new().expect("temporary directory");
    let mut app = app_in(dir.path());
    assert!(!actions::handle(
        &mut app,
        Action::File(FileCommand::Attributes)
    ));
    app = app_on_host(dir.path(), &[], None);
    assert!(!actions::handle(&mut app, Action::Quit));
    assert!(
        matches!(context::operation_target(&app), Ok(RemoteTarget::Panel { side: PanelSide::Left, path, .. }) if path == RemotePath::root())
    );
}
