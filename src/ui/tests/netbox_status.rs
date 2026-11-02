//! Unit tests: a `NetBox` panel's info row is the status line, as a file
//! panel's is — a refused dialog, F7, a file that did not load were said
//! nowhere — and it gives the row back once the cursor moves on.

use tempfile::TempDir;

use super::netbox_sessions::{draw, sessions_app};
use crate::models::{DirEntry, PanelKind, RemotePanel, RemotePath, RowMotion};
use crate::remote::fake::FakeRemoteFs;
use crate::remote::RemoteLink;
use crate::status::Status;
use crate::test_support::netbox::{app_in, name};

const REFUSAL: &str = "NetBox: Directory: not an absolute path";

#[test]
fn the_active_sessions_panel_shows_the_status_in_its_info_row() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = sessions_app(&dir);
    app.set_status(Status::Error(REFUSAL.to_owned()));
    let (screen, _) = draw(&mut app, 100, 20);
    assert!(screen.contains(REFUSAL), "{screen}");
    assert!(!screen.contains("2 sessions"), "{screen}");
}

#[test]
fn moving_the_sessions_cursor_gives_the_row_back_to_the_count() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = sessions_app(&dir);
    app.set_status(Status::Error(REFUSAL.to_owned()));
    app.expire_status_on_move();
    let (screen, _) = draw(&mut app, 100, 20);
    assert!(screen.contains(REFUSAL), "seen once:\n{screen}");
    if let PanelKind::Sessions(panel) = &mut app.left_panel.kind {
        panel.move_cursor(RowMotion::Next);
    }
    assert_eq!(app.left_panel.shown_cursor(), Some(1));
    app.expire_status_on_move();
    let (screen, _) = draw(&mut app, 100, 20);
    assert!(!screen.contains(REFUSAL), "{screen}");
    assert!(screen.contains("2 sessions"), "{screen}");
}

#[test]
fn the_inactive_panel_keeps_its_count() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = sessions_app(&dir);
    app.active_panel = crate::models::PanelSide::Right;
    app.set_status(Status::Error(REFUSAL.to_owned()));
    let (screen, _) = draw(&mut app, 100, 20);
    assert!(screen.contains("2 sessions"), "{screen}");
    assert_eq!(screen.matches(REFUSAL).count(), 1, "{screen}");
}

#[test]
fn the_active_remote_panel_shows_the_status_in_its_info_row() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app_in(dir.path());
    let mut panel = RemotePanel::new(
        name("demo@example.test"),
        RemoteLink::new(Box::new(FakeRemoteFs::default())),
    );
    let path = RemotePath::parse("/var/www").expect("path");
    let shown = panel.show(path, Vec::<DirEntry>::new());
    assert_eq!(shown.unusable_names, 0);
    app.left_panel.kind = PanelKind::Remote(Box::new(panel));
    assert_eq!(
        app.left_panel.shown_cursor(),
        Some(0),
        "the remote list's own"
    );
    app.set_status(Status::Error("NetBox: no connection".to_owned()));
    let (screen, _) = draw(&mut app, 100, 20);
    assert!(screen.contains("NetBox: no connection"), "{screen}");
}
