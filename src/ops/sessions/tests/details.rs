//! Purpose: F3 shows every setting of a session, and what it leaves unset.

use super::*;

/// What F3 shows for the session `which`.
fn viewed(app: &mut crate::models::App, which: &str) -> String {
    view(app, &name(which)).expect("viewed");
    let Overlay::Viewer(viewer) = &app.overlay else {
        panic!("the viewer");
    };
    assert_eq!(viewer.title(), format!("NetBox: {which}"));
    viewer.lines.join("\n")
}

const KEY_SESSION_LINES: [&str; 9] = [
    "Name:      u@a.test",
    "Protocol:  SFTP",
    "Host:      a.test",
    "Port:      22",
    "User:      u",
    "Login:     key file /home/bob/.ssh/id",
    "Directory: /var/www",
    "Origin:    imported from /srv/x/demo.netbox",
    "Address:   sftp://u@a.test/var/www",
];

#[test]
fn f3_shows_every_setting_of_a_session() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app_in(dir.path());
    let mut by_key = session("a.test", Some("u"));
    by_key.auth = Auth::KeyFile(KeyFile::new("/home/bob/.ssh/id").expect("key"));
    by_key.remote_dir = Some(RemotePath::parse("/var/www").expect("dir"));
    by_key.origin = Origin::NetBoxFile {
        path: "/srv/x/demo.netbox".into(),
    };
    app.sessions.upsert(by_key);
    let shown = viewed(&mut app, "u@a.test");
    for line in KEY_SESSION_LINES {
        assert!(shown.contains(line), "{line} in\n{shown}");
    }
    assert!(matches!(
        view(&mut app, &name("nobody")),
        Err(SessionError::NotFound(_))
    ));
}

#[test]
fn f3_says_how_a_session_logs_in_and_what_it_leaves_unset() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app_in(dir.path());
    let mut by_agent = session("b.test", None);
    by_agent.auth = Auth::Agent;
    app.sessions.upsert(by_agent);
    app.sessions.upsert(session("c.test", None));
    let agent = viewed(&mut app, "b.test");
    for line in [
        "Login:     SSH agent",
        "User:      (none)",
        "Directory: (login directory)",
        "Origin:    made or changed here",
    ] {
        assert!(agent.contains(line), "{line} in\n{agent}");
    }
    let password = viewed(&mut app, "c.test");
    assert!(password.contains("Login:     password, asked for when connecting"));
}
