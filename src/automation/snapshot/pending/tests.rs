//! Purpose: Waiting follows finite application requests and stops at user decisions or idle lists.

use super::*;
use crate::models::{Panel, WslDialog, WslOperation, WslRequestId};
use tempfile::TempDir;

mod cleanup;
mod images;

fn application(root: &TempDir) -> App {
    App::with_panels(
        Panel::new(root.path().to_path_buf()).unwrap(),
        Panel::new(root.path().to_path_buf()).unwrap(),
    )
}

#[test]
fn remote_outbox_and_in_flight_requests_remain_pending_after_an_overlay_closes() {
    use crate::models::remote_work::{RemoteTarget, RemoteTask};
    let root = TempDir::new().unwrap();
    let mut app = application(&root);
    assert!(!is_pending(&app));
    app.remote_requests
        .queue(
            RemoteTarget::Connect {
                side: PanelSide::Left,
                session: crate::models::SessionName::new("fixture").unwrap(),
            },
            RemoteTask::Delete(Vec::new()),
        )
        .unwrap();
    assert!(is_pending(&app));
    let request = app.remote_requests.take().unwrap();
    app.overlay = Overlay::Help;
    assert!(is_pending(&app));
    assert!(!app.remote_requests.finish(request.id + 1));
    assert!(is_pending(&app));
    assert!(app.remote_requests.finish(request.id));
    assert!(!is_pending(&app));
}

#[test]
fn provider_reports_are_pending_only_until_ready() {
    use crate::models::token_usage::TokenUsagePanel;
    let root = TempDir::new().unwrap();
    let mut app = application(&root);
    for (state, pending) in [
        (UsageState::Queued, true),
        (UsageState::Loading, true),
        (UsageState::Ready, false),
    ] {
        let mut panel = TokenUsagePanel::new();
        panel.state = state;
        app.overlay = Overlay::TokenUsage(Box::new(panel));
        assert_eq!(is_pending(&app), pending);
    }
}

fn things_cases() -> [(ThingsState, bool); 5] {
    use crate::models::things::ThingsCatalog;
    let config = crate::things::Config::from_values(
        Some("https://fixture.invalid".into()),
        Some("fixture-token".into()),
    )
    .unwrap();
    [
        (ThingsState::Queued(config), true),
        (ThingsState::Loading, true),
        (ThingsState::Setup("fixture".into()), false),
        (ThingsState::Failed("fixture".into()), false),
        (
            ThingsState::Ready(ThingsCatalog::new(crate::things::Listing::new(
                Vec::new(),
                None,
            ))),
            false,
        ),
    ]
}

#[test]
fn things_stops_waiting_at_ready_setup_or_failure_without_fetching_anything() {
    let root = TempDir::new().unwrap();
    let mut app = application(&root);
    for (state, pending) in things_cases() {
        app.overlay = Overlay::Things(Box::new(crate::models::things::ThingsDialog::new(state)));
        assert_eq!(is_pending(&app), pending);
    }
}

#[test]
fn wsl_waits_for_the_outbox_and_busy_dialog_but_not_finished_outcomes() {
    let root = TempDir::new().unwrap();
    let mut app = application(&root);
    app.wsl_requests.enqueue(WslOperation::Refresh);
    assert!(is_pending(&app));
    assert_eq!(app.wsl_requests.take().len(), 1);
    assert!(!is_pending(&app));
    for (activity, pending) in [
        (
            WslActivity::Busy {
                id: WslRequestId::new(1),
                operation: WslOperation::Refresh,
            },
            true,
        ),
        (WslActivity::Ready, false),
        (WslActivity::Notice("done".into()), false),
        (WslActivity::Failed("fixture".into()), false),
    ] {
        let mut dialog = WslDialog::new(Vec::new());
        dialog.activity = activity;
        app.overlay = Overlay::Wsl(Box::new(dialog));
        assert_eq!(is_pending(&app), pending);
    }
}

#[test]
fn a_permanent_task_sampler_and_ordinary_dialogs_do_not_make_wait_infinite() {
    let root = TempDir::new().unwrap();
    let mut app = application(&root);
    app.overlay = Overlay::Tasks(Box::default());
    assert!(!is_pending(&app));
    app.overlay = Overlay::Help;
    assert!(!is_pending(&app));
}
