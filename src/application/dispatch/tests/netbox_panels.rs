//! A `NetBox` panel through the real dispatcher: its keys are asked first,
//! and F5/F6 from a file panel refuse it as the destination.

use std::fs;

use ratatui::Terminal;
use tempfile::TempDir;

use super::*;
use crate::models::{PanelKind, PanelSide};
use crate::sessions::SessionStore;
use crate::test_support::netbox::{app_in, app_on_sessions, session};

fn terminal() -> Terminal<WritableTestBackend> {
    Terminal::new(WritableTestBackend::new(80, 24)).expect("test terminal")
}

/// A file under the cursor of the active left panel, the right panel showing
/// the sessions list.
fn beside_the_sessions(dir: &TempDir) -> App {
    fs::write(dir.path().join("a.txt"), "a").expect("write");
    let mut app = app_in(dir.path());
    let index = app
        .left_panel
        .entries
        .iter()
        .position(|entry| entry.name == "a.txt")
        .expect("listed");
    app.left_panel.select(Some(index));
    let _ = crate::ops::sessions::open_on(&mut app, PanelSide::Right);
    app
}

#[test]
fn f5_and_f6_refuse_a_sessions_list_as_the_destination() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = beside_the_sessions(&dir);
    let mut terminal = terminal();
    for code in [KeyCode::F(5), KeyCode::F(6)] {
        dispatch_key(&mut terminal, &mut app, key_event(code));
        assert!(!app.is_overlay_active(), "{code:?}: no copy prompt");
        assert_eq!(
            app.status_text(),
            "F5 and F6 need a folder in the other panel; it shows NetBox sessions or a host"
        );
    }
}

#[test]
fn f5_between_two_file_panels_still_asks_where_to() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = beside_the_sessions(&dir);
    app.right_panel.kind = PanelKind::Files;
    dispatch_key(&mut terminal(), &mut app, key_event(KeyCode::F(5)));
    assert!(matches!(app.overlay, Overlay::CopyPrompt(_)));
}

#[test]
fn the_dispatcher_asks_the_sessions_list_before_the_file_actions() {
    let dir = TempDir::new().expect("temp dir");
    let mut store = SessionStore::default();
    store.upsert(session("a.test", None));
    let mut app = app_on_sessions(dir.path(), store);
    let mut terminal = terminal();
    dispatch_key(&mut terminal, &mut app, key_event(KeyCode::Down));
    dispatch_key(&mut terminal, &mut app, key_event(KeyCode::F(8)));
    assert!(
        matches!(app.overlay, Overlay::Question(_)),
        "the session's F8"
    );
    app.close_overlay();
    dispatch_key(&mut terminal, &mut app, key_event(KeyCode::Tab));
    assert_eq!(
        app.active_panel,
        PanelSide::Right,
        "Tab still switches panels"
    );
}
