//! Unit tests: the sessions list is `..` then the sessions by name, follows
//! the store without losing its place, and takes a click on a drawn row.

use super::{SessionRow, SessionsPanel};
use crate::models::{HostName, Port, Protocol, RowMotion, Session, SessionName, UserName};
use crate::sessions::SessionStore;

fn session(host: &str, user: Option<&str>) -> Session {
    Session::new(
        Protocol::Sftp,
        HostName::new(host).expect("host"),
        Port::SSH,
        user.map(|user| UserName::new(user).expect("user")),
    )
}

fn store(sessions: Vec<Session>) -> SessionStore {
    let mut store = SessionStore::default();
    for session in sessions {
        store.upsert(session);
    }
    store
}

fn name(text: &str) -> SessionName {
    SessionName::new(text).expect("name")
}

#[test]
fn the_rows_are_the_parent_then_the_sessions_by_name() {
    let mut renamed = session("b.test", Some("u"));
    renamed.name = name("alpha");
    let panel = SessionsPanel::from_store(&store(vec![session("z.test", Some("u")), renamed]));
    assert_eq!(
        panel.rows(),
        [
            SessionRow::Parent,
            SessionRow::Session {
                name: name("alpha"),
                login: Some("u@b.test".to_owned()),
            },
            SessionRow::Session {
                name: name("u@z.test"),
                login: None,
            },
        ]
    );
    assert_eq!(panel.session_count(), 2);
    assert_eq!(panel.cursor(), 0);
    assert_eq!(panel.selected_name(), None, "`..` is not a session");
    assert_eq!(SessionRow::Parent.name(), None);
}

#[test]
fn an_empty_store_lists_only_the_way_back() {
    let panel = SessionsPanel::from_store(&SessionStore::default());
    assert_eq!(panel.rows(), [SessionRow::Parent]);
    assert_eq!(panel.session_count(), 0);
}

#[test]
fn reload_keeps_the_cursor_on_its_session_or_on_its_row() {
    let mut sessions = store(vec![session("a.test", None), session("c.test", None)]);
    let mut panel = SessionsPanel::from_store(&sessions);
    panel.move_cursor(RowMotion::Last);
    assert_eq!(panel.selected_name(), Some(&name("c.test")));

    sessions.upsert(session("b.test", None));
    panel.reload(&sessions);
    assert_eq!(
        panel.selected_name(),
        Some(&name("c.test")),
        "followed its session"
    );

    sessions.remove(&name("c.test"));
    panel.reload(&sessions);
    assert_eq!(
        panel.cursor(),
        2,
        "the same row, clamped to the shorter list"
    );
    assert_eq!(panel.selected_name(), Some(&name("b.test")));

    panel.move_cursor(RowMotion::First);
    panel.reload(&SessionStore::default());
    assert_eq!(panel.cursor(), 0);
}

#[test]
fn focus_moves_to_a_listed_session_and_ignores_an_unknown_one() {
    let mut panel = SessionsPanel::from_store(&store(vec![
        session("a.test", None),
        session("b.test", None),
    ]));
    panel.focus(&name("b.test"));
    assert_eq!(panel.cursor(), 2);
    panel.focus(&name("nobody"));
    assert_eq!(panel.cursor(), 2);
}

#[test]
fn a_click_selects_the_drawn_row_under_it() {
    let mut panel = SessionsPanel::from_store(&store(vec![
        session("a.test", None),
        session("b.test", None),
    ]));
    assert_eq!(panel.select_visible(1), Some(1));
    assert_eq!(panel.cursor(), 1);
    panel.scroll = 1;
    assert_eq!(
        panel.select_visible(1),
        Some(2),
        "rows are counted from the scroll"
    );
    assert_eq!(panel.select_visible(2), None, "below the last row");
    assert_eq!(panel.cursor(), 2, "a miss leaves the cursor");
}
