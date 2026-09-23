//! Unit tests: the sessions list's keys — moving, Enter, the session keys —
//! and the keys it refuses or hands back.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tempfile::TempDir;

use super::process;
use crate::models::{
    App, EditPurpose, InputPurpose, Overlay, PanelKind, QuestionPurpose, SessionsPanel,
};
use crate::sessions::SessionStore;
use crate::test_support::netbox::{app_in, app_on_sessions, name, session};

fn bare(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn app(dir: &TempDir) -> App {
    let mut store = SessionStore::default();
    store.upsert(session("a.test", None));
    store.upsert(session("b.test", None));
    app_on_sessions(dir.path(), store)
}

fn list(app: &App) -> &SessionsPanel {
    let PanelKind::Sessions(panel) = &app.left_panel.kind else {
        panic!("the sessions list");
    };
    panel
}

/// Move to the first session.
fn on_first_session(app: &mut App) {
    assert!(process(app, bare(KeyCode::Down)));
    assert_eq!(list(app).selected_name(), Some(&name("a.test")));
}

#[test]
fn the_list_keys_move_the_cursor() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app(&dir);
    assert!(process(&mut app, bare(KeyCode::End)));
    assert_eq!(list(&app).cursor(), 2);
    assert!(process(&mut app, bare(KeyCode::PageUp)));
    assert_eq!(list(&app).cursor(), 0);
    assert!(process(&mut app, bare(KeyCode::PageDown)));
    assert_eq!(list(&app).cursor(), 2);
    assert!(process(&mut app, bare(KeyCode::Home)));
    assert!(process(&mut app, bare(KeyCode::Down)));
    assert!(process(&mut app, bare(KeyCode::Up)));
    assert_eq!(list(&app).cursor(), 0);
}

#[test]
fn enter_on_the_parent_row_returns_to_the_files() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app(&dir);
    assert!(process(&mut app, bare(KeyCode::Enter)));
    assert_eq!(app.left_panel.kind, PanelKind::Files);
}

#[test]
fn enter_on_a_session_connects_and_this_build_says_it_cannot() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app(&dir);
    on_first_session(&mut app);
    assert!(process(&mut app, bare(KeyCode::Enter)));
    assert!(
        app.status_text()
            .starts_with("Error: NetBox: a.test: this build cannot connect"),
        "{}",
        app.status_text()
    );
    assert!(app.left_panel.kind.is_netbox(), "the list stays");
}

#[test]
fn the_session_keys_open_their_dialogs_on_the_session() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app(&dir);
    on_first_session(&mut app);

    assert!(process(&mut app, bare(KeyCode::F(3))));
    assert_eq!(
        app.get_viewer().map(crate::models::Viewer::title),
        Some("NetBox: a.test")
    );
    assert!(process(&mut app, bare(KeyCode::F(4))));
    assert!(matches!(
        &app.overlay,
        Overlay::SessionEdit(dialog) if dialog.purpose() == &EditPurpose::Edit(name("a.test"))
    ));
    assert!(process(&mut app, bare(KeyCode::F(5))));
    let Overlay::InputPrompt(prompt) = &app.overlay else {
        panic!("the copy asks for a name");
    };
    assert_eq!(
        prompt.purpose(),
        &InputPurpose::DuplicateSession(name("a.test"))
    );
    assert_eq!(prompt.input().text(), "a.test", "offered as the original's");
    assert!(process(&mut app, bare(KeyCode::F(8))));
    assert!(matches!(
        &app.overlay,
        Overlay::Question(question)
            if question.purpose() == &QuestionPurpose::DeleteSession(name("a.test"))
    ));
}

#[test]
fn shift_f4_makes_a_new_session_wherever_the_cursor_is() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app(&dir);
    assert!(process(
        &mut app,
        KeyEvent::new(KeyCode::F(4), KeyModifiers::SHIFT)
    ));
    assert!(matches!(
        &app.overlay,
        Overlay::SessionEdit(dialog) if dialog.purpose() == &EditPurpose::Create
    ));
}

#[test]
fn the_session_keys_do_nothing_on_the_parent_row() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app(&dir);
    for code in [KeyCode::F(3), KeyCode::F(4), KeyCode::F(5), KeyCode::F(8)] {
        assert!(process(&mut app, bare(code)), "{code:?}");
        assert!(!app.is_overlay_active(), "{code:?}");
    }
    assert!(app.left_panel.kind.is_netbox());
}

#[test]
fn a_key_for_the_hidden_files_is_refused_and_a_program_key_passes() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app(&dir);
    assert!(process(&mut app, bare(KeyCode::F(7))));
    assert_eq!(
        app.status_text(),
        "NetBox: this key works on files; press Enter on `..` to return to them"
    );
    assert!(process(
        &mut app,
        KeyEvent::new(KeyCode::F(3), KeyModifiers::ALT)
    ));
    assert!(!process(&mut app, bare(KeyCode::Tab)));
    assert!(!process(
        &mut app,
        KeyEvent::new(KeyCode::F(1), KeyModifiers::ALT)
    ));
}

#[test]
fn another_kind_of_panel_is_not_the_lists() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app_in(dir.path());
    assert!(!process(&mut app, bare(KeyCode::F(8))));
    assert!(!app.is_overlay_active());
}
