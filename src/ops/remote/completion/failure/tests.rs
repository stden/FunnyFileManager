//! Purpose: Connect failures do not replace an already open overlay.

use crate::models::remote_work::{RemoteRequest, RemoteTarget, RemoteTask};
use crate::models::{Overlay, PanelSide};
use crate::remote::{Credentials, RemoteError};
use crate::test_support::netbox::{app_in, session};

#[test]
fn connect_failure_with_active_overlay_preserves_overlay_and_reports_error() {
    let dir = tempfile::TempDir::new().expect("temp dir");
    let mut app = app_in(dir.path());
    app.set_overlay(Overlay::Help);
    let stored = session("example.test", Some("alice"));
    let request = RemoteRequest {
        id: 1,
        target: RemoteTarget::DirectConnect {
            side: PanelSide::Left,
            session: stored.name.clone(),
        },
        task: RemoteTask::Connect {
            session: stored,
            credentials: Credentials::default(),
        },
    };

    super::apply(&mut app, request, &RemoteError::AuthenticationRequired);

    assert!(matches!(app.overlay, Overlay::Help));
    assert!(app.status_text().contains("NetBox:"));
}
