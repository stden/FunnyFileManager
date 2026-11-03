use super::{input, task, RemoteError, RemoteInput};
use crate::models::remote_work::RemoteTarget;
use crate::models::{PanelKind, PanelSide, SessionsPanel};
use crate::test_support::netbox::{app_in, session};

#[test]
fn selection_dialog_cannot_create_a_remote_worker_job() {
    assert!(matches!(
        task(RemoteInput::Selection { select: true }, "*"),
        Err(RemoteError::InvalidInput("selection is not a job"))
    ));
}

#[test]
fn selection_input_against_connect_target_is_a_clean_noop() {
    let dir = tempfile::TempDir::new().expect("tempdir");
    let mut app = app_in(dir.path());
    let stored = session("example.test", Some("u"));
    let name = stored.name.clone();
    app.sessions.upsert(stored);
    let mut list = SessionsPanel::from_store(&app.sessions);
    list.focus(&name);
    app.left_panel.kind = PanelKind::Sessions(Box::new(list));
    input(
        &mut app,
        RemoteTarget::Connect {
            side: PanelSide::Left,
            session: name,
        },
        RemoteInput::Selection { select: true },
        "*",
    )
    .expect("noop selection on connect target");
}
