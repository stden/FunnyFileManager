//! Purpose: Unit tests for local/remote panel location resolution and target matching.

use super::{directory, operation_target, other, selected, sources};
use crate::models::{App, Panel, PanelKind, PanelSide, RemotePanel, SessionName, SessionsPanel};
use crate::remote::fake::FakeRemoteFs;
use crate::remote::{RemoteError, RemoteLink};
use crate::sessions::SessionStore;

#[test]
fn context_resolves_files_panel_refuses_sessions_panel_and_finds_right_remote() {
    let temp = tempfile::TempDir::new().unwrap();
    std::fs::write(temp.path().join("a.txt"), b"x").unwrap();
    let left = Panel::new(temp.path().to_path_buf()).unwrap();
    let right = Panel::new(temp.path().to_path_buf()).unwrap();
    let mut app = App::with_panels(left, right);
    assert!(app.left_panel.select_named("a.txt"));
    assert!(directory(&app.left_panel).is_ok());
    assert_eq!(sources(&app.left_panel).unwrap().len(), 1);
    assert!(selected(&app.left_panel).unwrap().is_some());
    assert!(matches!(
        operation_target(&app),
        Err(RemoteError::Failed(_))
    ));

    let mut sessions = Panel::from_entries(temp.path().to_owned(), Vec::new());
    let store = SessionStore::load_from(temp.path().join("none.yml"));
    sessions.kind = PanelKind::Sessions(Box::new(SessionsPanel::from_store(&store)));
    assert!(directory(&sessions).is_err());
    assert!(sources(&sessions).is_err());
    assert!(selected(&sessions).is_err());

    let link = RemoteLink::new(Box::new(FakeRemoteFs::default()));
    app.right_panel.kind = PanelKind::Remote(Box::new(RemotePanel::new(
        SessionName::new("srv").unwrap(),
        link,
    )));
    assert!(operation_target(&app).is_ok());
    assert_eq!(other(PanelSide::Right), PanelSide::Left);
}
