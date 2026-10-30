//! Unit tests: a session opens its host through the seam in its start
//! directory, Enter walks the host's directories and leaves at its root,
//! and every failure — no transport, a missing directory, a file, a hostile
//! name — reaches the status bar instead of moving the panel.

use tempfile::TempDir;

use super::{connect, enter, leave, list_into, open_with};
use crate::models::{App, DirEntry, EntryKind, PanelKind, RemotePanel, RemotePath, RowMotion};
use crate::remote::fake::FakeRemoteFs;
use crate::remote::{RemoteError, RemoteFs};
use crate::status::Status;
use crate::test_support::netbox::{app_in, app_on_host, name, session};

const FILES: [(&str, u64); 3] = [
    ("/var/www/index.html", 10),
    ("/var/log/syslog", 20),
    ("/etc/hostname", 5),
];

fn remote(app: &App) -> &RemotePanel {
    let PanelKind::Remote(panel) = &app.left_panel.kind else {
        panic!("the left panel browses the host");
    };
    panel
}

fn names(app: &App) -> Vec<String> {
    remote(app)
        .entries()
        .iter()
        .map(|e| e.name.clone())
        .collect()
}

/// Put the cursor on the row named `row` and press Enter.
fn enter_on(app: &mut App, row: &str) {
    let index = names(app)
        .iter()
        .position(|name| name == row)
        .expect("listed");
    if let PanelKind::Remote(panel) = &mut app.left_panel.kind {
        panel.move_cursor(RowMotion::First);
        for _ in 0..index {
            panel.move_cursor(RowMotion::Next);
        }
    }
    enter(app);
}

#[test]
fn a_session_opens_in_its_start_directory_or_at_the_root() {
    let dir = TempDir::new().expect("temp dir");
    let app = app_on_host(dir.path(), &FILES, Some("/var/www"));
    assert_eq!(remote(&app).path().as_str(), "/var/www");
    assert_eq!(names(&app), ["..", "index.html"]);
    assert_eq!(
        app.status,
        Status::CommandOutput("NetBox: sftp://demo@example.test/var/www".to_owned())
    );
    let at_root = app_on_host(dir.path(), &FILES, None);
    assert_eq!(names(&at_root), ["..", "etc", "var"]);
}

#[test]
fn a_start_directory_the_host_lacks_is_an_error_and_the_panel_stays() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app_in(dir.path());
    let mut start_missing = session("example.test", None);
    start_missing.remote_dir = RemotePath::parse("/nope").ok();
    let opened = open_with(
        &mut app,
        &start_missing,
        Box::new(FakeRemoteFs::with_files(&FILES)),
    );
    assert_eq!(
        opened,
        Err(RemoteError::NotFound(
            RemotePath::parse("/nope").expect("path")
        ))
    );
    assert_eq!(app.left_panel.kind, PanelKind::Files);
}

#[test]
fn enter_walks_down_and_back_up_and_leaves_at_the_root() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app_on_host(dir.path(), &FILES, None);
    enter_on(&mut app, "var");
    assert_eq!(remote(&app).path().as_str(), "/var");
    assert_eq!(names(&app), ["..", "log", "www"]);
    assert_eq!(app.status_text(), "NetBox: demo@example.test:/var");
    enter_on(&mut app, "log");
    assert_eq!(names(&app), ["..", "syslog"]);
    enter_on(&mut app, "..");
    assert_eq!(remote(&app).path().as_str(), "/var");
    enter_on(&mut app, "..");
    assert!(remote(&app).path().is_root());
    enter_on(&mut app, "..");
    assert_eq!(app.left_panel.kind, PanelKind::Files, "out of the host");
    assert_eq!(app.status_text(), "NetBox: left demo@example.test");
}

#[test]
fn enter_on_a_file_says_files_on_a_host_do_not_open() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app_on_host(dir.path(), &FILES, Some("/etc"));
    enter_on(&mut app, "hostname");
    assert_eq!(
        app.status,
        Status::Error(
            "NetBox: hostname: opening a file on a host is not part of this build".to_owned()
        )
    );
    assert_eq!(remote(&app).path().as_str(), "/etc", "the panel stays");
}

#[test]
fn a_directory_that_went_away_is_reported_and_the_listing_stays() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app_on_host(dir.path(), &FILES, None);
    list_into(&mut app, RemotePath::parse("/gone").expect("path"));
    assert_eq!(
        app.status,
        Status::Error("NetBox: no such remote directory: /gone".to_owned())
    );
    assert_eq!(names(&app), ["..", "etc", "var"]);
}

/// A host that lists a name holding `/` or spelled `..` cannot steer the
/// panel: the entry is left out and the user is told.
struct HostileHost;

impl RemoteFs for HostileHost {
    fn list(&mut self, _dir: &RemotePath) -> Result<Vec<DirEntry>, RemoteError> {
        let entry = |name: &str| DirEntry {
            name: name.to_owned(),
            kind: EntryKind::Directory,
            ..DirEntry::default()
        };
        Ok(vec![entry("../../etc"), entry(".."), entry("ok")])
    }
}

#[test]
fn names_that_could_steer_the_panel_are_left_out_and_reported() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app_in(dir.path());
    let host = session("example.test", None);
    open_with(&mut app, &host, Box::new(HostileHost)).expect("listed");
    assert_eq!(names(&app), ["..", "ok"]);
    assert_eq!(
        app.status,
        Status::Error(
            "NetBox: 2 entries left out: their names cannot be part of a path".to_owned()
        )
    );
    enter_on(&mut app, "ok");
    assert_eq!(remote(&app).path().as_str(), "/ok");
}

#[test]
fn connecting_in_this_build_reports_the_missing_transport() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app_in(dir.path());
    app.sessions.upsert(session("example.test", Some("demo")));
    connect(&mut app, &name("demo@example.test"));
    assert_eq!(
        app.status,
        Status::Error(format!(
            "NetBox: demo@example.test: {}",
            RemoteError::NoTransport(crate::models::Protocol::Sftp)
        ))
    );
    assert_eq!(app.left_panel.kind, PanelKind::Files);
    connect(&mut app, &name("nobody"));
    assert_eq!(
        app.status,
        Status::Error("NetBox: no session named nobody".to_owned())
    );
}

#[test]
fn the_remote_keys_do_nothing_on_another_kind_of_panel() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app_in(dir.path());
    app.set_status(Status::CommandOutput("before".to_owned()));
    enter(&mut app);
    leave(&mut app);
    list_into(&mut app, RemotePath::root());
    assert_eq!(app.left_panel.kind, PanelKind::Files);
    assert_eq!(app.status_text(), "before");
}
