//! Unit tests: the sessions panel opens on the side asked for and leaves on
//! `..`; every change to the store is saved before it shows, and a change
//! the file does not take leaves the store as it was.

use std::fs;
use std::io;

use tempfile::TempDir;

use super::{
    back_to_files, back_to_files_on, create, delete, duplicate, edit, execute, install, open,
    open_on, refresh, report, save_edit, view, SessionError,
};
use crate::command::SessionsCommand;
use crate::models::{
    Auth, EditPurpose, Field, InvalidField, KeyFile, Origin, Overlay, PanelKind, PanelSide,
    Problem, RemotePath, RowMotion, SessionEdit,
};
use crate::sessions::import::ImportError;
use crate::sessions::{SessionStore, StoreError, FILE_NAME};
use crate::status::Status;
use crate::test_support::netbox::{
    app_in, app_on_sessions, file_store, name, open_empty_sessions, session,
};

fn sessions_panel(app: &crate::models::App) -> &crate::models::SessionsPanel {
    let PanelKind::Sessions(panel) = &app.left_panel.kind else {
        panic!("the left panel lists the sessions");
    };
    panel
}

fn stored_names(dir: &TempDir) -> Vec<String> {
    let store = SessionStore::load_from(dir.path().join(FILE_NAME));
    store.sorted().iter().map(|s| s.name.to_string()).collect()
}

/// The Shift+F4 dialog with `host` typed in.
fn dialog_for(host: &str) -> SessionEdit {
    let mut dialog = SessionEdit::create();
    dialog.host = host.to_owned();
    dialog
}

#[test]
fn open_turns_the_named_panel_into_the_sessions_list_and_counts_them() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app_in(dir.path());
    app.sessions.upsert(session("a.test", None));

    let status = open_on(&mut app, PanelSide::Right);
    assert!(app.right_panel.kind.is_netbox());
    assert!(
        !app.left_panel.kind.is_netbox(),
        "the other side is untouched"
    );
    assert_eq!(
        status,
        Status::CommandOutput("NetBox: 1 stored session".to_owned())
    );

    app.sessions.upsert(session("b.test", None));
    open(&mut app);
    assert_eq!(sessions_panel(&app).session_count(), 2, "the active panel");
    assert_eq!(app.status_text(), "NetBox: 2 stored sessions");
}

#[test]
fn a_damaged_store_says_so_when_its_panel_opens() {
    let dir = TempDir::new().expect("temp dir");
    fs::write(dir.path().join(FILE_NAME), "{").expect("write");
    let mut app = app_in(dir.path());
    app.sessions = SessionStore::load_from(dir.path().join(FILE_NAME));
    let status = open_on(&mut app, PanelSide::Left);
    assert_eq!(
        status,
        Status::Error(format!("NetBox: {}", StoreError::Damaged))
    );
    assert!(app.left_panel.kind.is_netbox());
}

#[test]
fn back_to_files_only_leaves_a_netbox_panel() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app_in(dir.path());
    app.left_panel.kind = PanelKind::Tree(Box::default());
    back_to_files(&mut app);
    assert!(app.left_panel.kind.is_tree(), "a tree is not NetBox's");
    open(&mut app);
    back_to_files(&mut app);
    assert_eq!(app.left_panel.kind, PanelKind::Files);
    open_empty_sessions(&mut app, PanelSide::Right);
    back_to_files_on(&mut app, PanelSide::Right);
    assert_eq!(app.right_panel.kind, PanelKind::Files);
}

#[test]
fn refresh_follows_the_store_in_every_sessions_panel() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app_on_sessions(dir.path(), SessionStore::default());
    open_empty_sessions(&mut app, PanelSide::Right);
    app.sessions.upsert(session("a.test", None));
    refresh(&mut app);
    assert_eq!(sessions_panel(&app).session_count(), 1);
    let PanelKind::Sessions(right) = &app.right_panel.kind else {
        panic!("the right panel lists the sessions");
    };
    assert_eq!(right.session_count(), 1);
}

#[test]
fn shift_f4_opens_an_empty_dialog_and_f4_one_filled_from_the_session() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app_in(dir.path());
    create(&mut app);
    let Overlay::SessionEdit(dialog) = &app.overlay else {
        panic!("the session dialog");
    };
    assert_eq!(dialog.purpose(), &EditPurpose::Create);

    app.sessions.upsert(session("a.test", Some("u")));
    edit(&mut app, &name("u@a.test")).expect("edit");
    let Overlay::SessionEdit(dialog) = &app.overlay else {
        panic!("the session dialog");
    };
    assert_eq!(dialog.host, "a.test");
    assert!(matches!(
        edit(&mut app, &name("nobody")),
        Err(SessionError::NotFound(missing)) if missing == name("nobody")
    ));
}

#[test]
fn saving_a_new_session_stores_it_and_puts_the_cursor_on_it() {
    let dir = TempDir::new().expect("temp dir");
    let store = file_store(dir.path(), vec![session("z.test", None)]);
    let mut app = app_on_sessions(dir.path(), store);
    let dialog = dialog_for("a.test");
    app.set_overlay(Overlay::SessionEdit(Box::new(dialog.clone())));
    save_edit(&mut app, &dialog).expect("saved");
    assert!(!app.is_overlay_active(), "the dialog closes");
    assert_eq!(stored_names(&dir), ["a.test", "z.test"]);
    assert_eq!(sessions_panel(&app).selected_name(), Some(&name("a.test")));
    assert_eq!(app.status_text(), "NetBox: saved a.test");
}

#[test]
fn a_new_session_may_not_take_a_stored_name() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app_on_sessions(
        dir.path(),
        file_store(dir.path(), vec![session("a.test", None)]),
    );
    let dialog = dialog_for("a.test");
    app.set_overlay(Overlay::SessionEdit(Box::new(dialog.clone())));
    let error = save_edit(&mut app, &dialog).expect_err("taken");
    assert!(matches!(&error, SessionError::NameTaken(taken) if *taken == name("a.test")));
    assert!(app.is_overlay_active(), "the dialog stays for a correction");
    assert_eq!(app.sessions.len(), 1);
}

#[test]
fn editing_a_session_replaces_it_and_a_rename_may_not_take_another_name() {
    let dir = TempDir::new().expect("temp dir");
    let store = file_store(
        dir.path(),
        vec![session("a.test", None), session("b.test", None)],
    );
    let mut app = app_on_sessions(dir.path(), store);
    let original = app.sessions.find(&name("a.test")).cloned().expect("stored");

    let mut dialog = SessionEdit::from_session(&original);
    dialog.port = "2222".to_owned();
    save_edit(&mut app, &dialog).expect("the same name");
    assert_eq!(
        app.sessions.find(&name("a.test")).map(|s| s.port.get()),
        Some(2222)
    );

    dialog.name = "web".to_owned();
    save_edit(&mut app, &dialog).expect("renamed");
    assert_eq!(stored_names(&dir), ["b.test", "web"]);

    let mut onto = SessionEdit::from_session(app.sessions.find(&name("web")).expect("web"));
    onto.name = "b.test".to_owned();
    assert!(matches!(
        save_edit(&mut app, &onto),
        Err(SessionError::NameTaken(_))
    ));
    assert_eq!(stored_names(&dir), ["b.test", "web"]);
}

#[test]
fn an_invalid_dialog_is_refused_with_the_field() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app_in(dir.path());
    let error = save_edit(&mut app, &SessionEdit::create()).expect_err("no host");
    assert!(matches!(
        error,
        SessionError::Invalid(field) if field == InvalidField::new(Field::Host, Problem::Empty)
    ));
    assert!(app.sessions.is_empty());
}

/// A change the file does not take is not made at all.
#[test]
fn a_change_that_cannot_be_saved_leaves_the_store_as_it_was() {
    let dir = TempDir::new().expect("temp dir");
    fs::write(dir.path().join(FILE_NAME), "{").expect("write");
    let mut app = app_in(dir.path());
    app.sessions = SessionStore::load_from(dir.path().join(FILE_NAME));
    let dialog = dialog_for("a.test");
    let error = save_edit(&mut app, &dialog).expect_err("damaged");
    assert!(matches!(error, SessionError::Store(StoreError::Damaged)));
    assert!(app.sessions.is_empty());
    assert_eq!(
        fs::read_to_string(dir.path().join(FILE_NAME)).expect("read"),
        "{"
    );
}

#[test]
fn a_copy_is_the_users_own_under_its_new_name() {
    let dir = TempDir::new().expect("temp dir");
    let mut imported = session("a.test", Some("u"));
    imported.origin = Origin::NetBoxFile("/srv/x/demo.netbox".into());
    let mut app = app_on_sessions(dir.path(), file_store(dir.path(), vec![imported]));

    duplicate(&mut app, &name("u@a.test"), " copy ").expect("copied");
    let copy = app.sessions.find(&name("copy")).expect("the copy");
    assert!(copy.origin.is_manual());
    assert_eq!(copy.host.as_str(), "a.test");
    assert_eq!(stored_names(&dir), ["copy", "u@a.test"]);
    assert_eq!(sessions_panel(&app).selected_name(), Some(&name("copy")));
    assert_eq!(app.status_text(), "NetBox: copied u@a.test to copy");

    assert!(matches!(
        duplicate(&mut app, &name("u@a.test"), "copy"),
        Err(SessionError::NameTaken(_))
    ));
    assert!(matches!(
        duplicate(&mut app, &name("u@a.test"), "  "),
        Err(SessionError::Invalid(_))
    ));
    assert!(matches!(
        duplicate(&mut app, &name("nobody"), "other"),
        Err(SessionError::NotFound(_))
    ));
}

#[test]
fn delete_removes_the_session_from_the_store_and_the_panel() {
    let dir = TempDir::new().expect("temp dir");
    let store = file_store(
        dir.path(),
        vec![session("a.test", None), session("b.test", None)],
    );
    let mut app = app_on_sessions(dir.path(), store);
    if let PanelKind::Sessions(panel) = &mut app.left_panel.kind {
        panel.move_cursor(RowMotion::Last);
    }
    delete(&mut app, &name("b.test")).expect("deleted");
    assert_eq!(stored_names(&dir), ["a.test"]);
    assert_eq!(sessions_panel(&app).selected_name(), Some(&name("a.test")));
    assert_eq!(app.status_text(), "NetBox: deleted b.test");
    assert!(matches!(
        delete(&mut app, &name("b.test")),
        Err(SessionError::NotFound(_))
    ));
}

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
    by_key.origin = Origin::NetBoxFile("/srv/x/demo.netbox".into());
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

#[test]
fn a_failure_reaches_the_status_bar_under_netbox_s_name() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app_in(dir.path());
    app.set_status(Status::CommandOutput("before".to_owned()));
    report(&mut app, Ok(()));
    assert_eq!(app.status_text(), "before");
    report(&mut app, Err(SessionError::NotFound(name("x"))));
    assert_eq!(
        app.status,
        Status::Error("NetBox: no session named x".to_owned())
    );
}

#[test]
fn every_error_reads_as_one_line() {
    let texts = [
        (SessionError::NotFound(name("a")), "no session named a"),
        (
            SessionError::NameTaken(name("a")),
            "a session named a already exists",
        ),
        (
            SessionError::Invalid(InvalidField::new(Field::Port, Problem::NotAPort)),
            "Port: must be a number from 1 to 65535",
        ),
        (
            SessionError::Import(ImportError::NotNetBox),
            "not a NetBox session export",
        ),
        (
            SessionError::NoImportSource,
            "name a .netbox file, or put a file panel's cursor on one",
        ),
    ];
    for (error, text) in texts {
        assert_eq!(error.to_string(), text);
    }
    let store = SessionError::Store(StoreError::Damaged);
    assert_eq!(store.to_string(), StoreError::Damaged.to_string());
    let from: SessionError = InvalidField::new(Field::Name, Problem::Empty).into();
    assert!(matches!(from, SessionError::Invalid(_)));
}

#[test]
fn the_sessions_command_opens_the_panel_or_imports() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app_in(dir.path());
    execute(&mut app, SessionsCommand::Open).expect("opened");
    assert!(app.left_panel.kind.is_netbox());
    let error = execute(
        &mut app,
        SessionsCommand::ImportNetBox(Some(dir.path().join("missing.netbox"))),
    )
    .expect_err("no such file");
    assert_eq!(error.kind(), io::ErrorKind::Other);
    assert!(
        error
            .to_string()
            .starts_with("NetBox: cannot read the file: "),
        "{error}"
    );
}

#[test]
fn installing_a_store_that_did_not_load_says_so() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app_in(dir.path());
    app.set_status(Status::CommandOutput("before".to_owned()));
    install(
        &mut app,
        file_store(dir.path(), vec![session("a.test", None)]),
    );
    assert_eq!(app.sessions.len(), 1);
    assert_eq!(app.status_text(), "before", "a good store says nothing");

    fs::write(dir.path().join(FILE_NAME), "{").expect("write");
    install(
        &mut app,
        SessionStore::load_from(dir.path().join(FILE_NAME)),
    );
    assert!(app.sessions.is_damaged());
    assert_eq!(
        app.status,
        Status::Error(format!("NetBox: {}", StoreError::Damaged))
    );
}
