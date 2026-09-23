//! Purpose: `NetBox` fixtures shared by the sessions, remote-panel and dialog
//! tests — made-up sessions, a store, and an app on a temporary directory —
//! so each test states only what it is about.

use std::path::Path;

use crate::models::{
    App, HostName, Panel, PanelKind, Port, Protocol, RemotePath, Session, SessionName,
    SessionsPanel, UserName,
};
use crate::remote::fake::FakeRemoteFs;
use crate::sessions::{SessionStore, FILE_NAME};

/// An SFTP session to `host` on port 22, named `user@host` or `host`.
pub fn session(host: &str, user: Option<&str>) -> Session {
    Session::new(
        Protocol::Sftp,
        HostName::new(host).expect("a valid fixture host"),
        Port::SSH,
        user.map(|user| UserName::new(user).expect("a valid fixture user")),
    )
}

pub fn name(text: &str) -> SessionName {
    SessionName::new(text).expect("a valid fixture name")
}

/// A store kept in `dir`, holding `sessions`, already saved there.
pub fn file_store(dir: &Path, sessions: Vec<Session>) -> SessionStore {
    let mut store = SessionStore::load_from(dir.join(FILE_NAME));
    for session in sessions {
        store.upsert(session);
    }
    store.save().expect("the fixture store saves");
    store
}

/// Both panels on `dir`, the left one active.
pub fn app_in(dir: &Path) -> App {
    let panel = Panel::new(dir.to_path_buf()).expect("a readable fixture directory");
    App::with_panels(panel.clone(), panel)
}

/// [`app_in`] with `store`, the active panel showing it.
pub fn app_on_sessions(dir: &Path, store: SessionStore) -> App {
    let mut app = app_in(dir);
    app.sessions = store;
    app.left_panel.kind = PanelKind::Sessions(Box::new(SessionsPanel::from_store(&app.sessions)));
    app
}

/// [`app_in`] with the active panel browsing a fake host holding `files`,
/// through the session `demo@example.test`, which starts in `start`.
pub fn app_on_host(dir: &Path, files: &[(&str, u64)], start: Option<&str>) -> App {
    let mut session = session("example.test", Some("demo"));
    session.remote_dir = start.map(|dir| RemotePath::parse(dir).expect("a valid fixture dir"));
    let mut app = app_in(dir);
    app.sessions.upsert(session.clone());
    crate::ops::remote::open_with(
        &mut app,
        &session,
        Box::new(FakeRemoteFs::with_files(files)),
    )
    .expect("the fake host lists its start directory");
    app
}
