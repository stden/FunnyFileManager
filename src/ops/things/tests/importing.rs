//! Unit tests: an import saves what was chosen through the store, takes only the
//! marked machines when any are marked, keeps a manual entry, refreshes the
//! entries an earlier import wrote, and says when there was nothing to take.

use tempfile::TempDir;

use super::super::submit;
use super::{app, console_lines, loaded, machine, two_machines};
use crate::models::things::ThingsState;
use crate::models::{Origin, Overlay, RowMotion};
use crate::sessions::{SessionStore, FILE_NAME};
use crate::status::Status;
use crate::test_support::netbox::{name, session};

#[test]
fn submit_imports_every_shown_machine_saves_them_and_closes_the_dialog() {
    let dir = TempDir::new().expect("temp");
    let mut app = app(&dir, Vec::new());
    loaded(&mut app, two_machines(), None);
    submit(&mut app);
    assert!(!app.overlay.is_active());
    assert_eq!(
        app.status,
        Status::CommandOutput("Things: 2 imported, 0 skipped".to_owned())
    );
    let saved = SessionStore::load_from(dir.path().join(FILE_NAME));
    let web = saved.find(&name("web-01")).expect("web-01 is saved");
    assert_eq!(web.host.as_str(), "web-01.example.com");
    assert_eq!(
        web.origin,
        Origin::Things {
            machine: "id-web-01".to_owned()
        }
    );
}

#[test]
fn submit_takes_only_the_marked_machines_when_any_are_marked() {
    let dir = TempDir::new().expect("temp");
    let mut app = app(&dir, Vec::new());
    loaded(&mut app, two_machines(), None);
    if let Overlay::Things(dialog) = &mut app.overlay {
        if let ThingsState::Ready(catalog) = &mut dialog.state {
            catalog.move_cursor(RowMotion::Next);
            catalog.toggle_mark();
        }
    }
    submit(&mut app);
    assert_eq!(
        app.status,
        Status::CommandOutput("Things: 1 imported, 0 skipped".to_owned())
    );
    assert!(app.sessions.find(&name("web-01")).is_some());
    assert!(app.sessions.find(&name("db-01")).is_none());
}

#[test]
fn a_manual_entry_with_the_same_name_is_kept_and_reported_on_the_console() {
    let dir = TempDir::new().expect("temp");
    let mut manual = session("hand.example.com", None);
    manual.name = name("web-01");
    let mut app = app(&dir, vec![manual]);
    loaded(&mut app, two_machines(), None);
    submit(&mut app);
    assert_eq!(
        app.status,
        Status::CommandOutput("Things: 1 imported, 1 skipped".to_owned())
    );
    let transcript = console_lines(&app);
    assert!(
        transcript
            .iter()
            .any(|line| line == "Things: skipped web-01: already stored and changed by hand; kept"),
        "{transcript:?}"
    );
    let kept = app.sessions.find(&name("web-01")).expect("kept");
    assert_eq!(kept.host.as_str(), "hand.example.com");
}

#[test]
fn a_second_import_refreshes_the_entries_the_first_one_wrote() {
    let dir = TempDir::new().expect("temp");
    let mut app = app(&dir, Vec::new());
    loaded(&mut app, vec![machine("web-01", "old.example.com")], None);
    submit(&mut app);
    loaded(&mut app, vec![machine("web-01", "new.example.com")], None);
    submit(&mut app);
    assert_eq!(
        app.status,
        Status::CommandOutput("Things: 1 imported, 0 skipped".to_owned())
    );
    let web = app.sessions.find(&name("web-01")).expect("web-01");
    assert_eq!(web.host.as_str(), "new.example.com");
}

#[test]
fn an_import_with_nothing_to_take_closes_the_dialog_and_says_so() {
    let dir = TempDir::new().expect("temp");
    let mut app = app(&dir, Vec::new());
    loaded(&mut app, Vec::new(), None);
    submit(&mut app);
    assert!(!app.overlay.is_active());
    assert_eq!(
        app.status,
        Status::Error("Things: no machines to import".to_owned())
    );
}
