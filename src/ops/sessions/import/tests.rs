//! Unit tests: an import finds its file — named, relative to a file panel,
//! or under a file panel's cursor — takes what it found into the store, and
//! reports the counts on the status bar and every skip in the transcript.

use std::fs;
use std::path::PathBuf;

use tempfile::TempDir;

use super::{prompt, run, submit, EXTENSION};
use crate::models::{App, InputPurpose, Overlay, PanelKind, SessionsPanel};
use crate::ops::sessions::SessionError;
use crate::sessions::import::ImportError;
use crate::sessions::{SessionStore, StoreError, FILE_NAME, MAX_FILE_BYTES};
use crate::test_support::netbox::{app_in, file_store, name, session};

const EXPORT: &str = "<NetBox><Sessions>\
    <Session name=\"web\"><HostName>web.test</HostName><Password>0A0B</Password></Session>\
    <Session name=\"bad\"><HostName>a b</HostName></Session>\
    </Sessions></NetBox>";

/// A directory holding `demo.netbox`, and an app on it whose cursor is on
/// that file.
fn setup() -> (TempDir, App) {
    let dir = TempDir::new().expect("temp dir");
    fs::write(dir.path().join(format!("demo.{EXTENSION}")), EXPORT).expect("write");
    let mut app = app_in(dir.path());
    app.sessions = SessionStore::load_from(dir.path().join(FILE_NAME));
    select(&mut app, "demo.netbox");
    (dir, app)
}

fn select(app: &mut App, name: &str) {
    let index = app
        .left_panel
        .entries
        .iter()
        .position(|entry| entry.name == name)
        .expect("listed");
    app.left_panel.select(Some(index));
}

fn transcript(app: &App) -> Vec<String> {
    app.console
        .visible(100)
        .iter()
        .map(|line| line.text().to_owned())
        .collect()
}

#[test]
fn a_named_file_is_imported_saved_and_reported() {
    let (dir, mut app) = setup();
    run(&mut app, Some(dir.path().join("demo.netbox"))).expect("imported");
    assert!(app.sessions.find(&name("web")).is_some());
    let saved = SessionStore::load_from(dir.path().join(FILE_NAME));
    assert!(saved.find(&name("web")).is_some(), "written to the file");
    assert_eq!(app.status_text(), "NetBox: 1 imported, 1 skipped");
    assert_eq!(
        transcript(&app),
        [
            "NetBox: skipped bad: Host: must not hold ' '",
            "NetBox: 1 saved password(s) not imported; a password is asked for when connecting",
        ]
    );
}

#[test]
fn a_relative_path_is_taken_from_the_file_panels_directory() {
    let (_dir, mut app) = setup();
    run(&mut app, Some(PathBuf::from("demo.netbox"))).expect("imported");
    assert_eq!(app.sessions.len(), 1);
}

#[test]
fn without_a_path_the_netbox_file_under_the_cursor_is_read() {
    let (_dir, mut app) = setup();
    run(&mut app, None).expect("imported");
    assert_eq!(app.sessions.len(), 1);
}

/// The F11 row is used over the sessions list, which hides the file list:
/// the other panel's cursor counts then.
#[test]
fn over_a_sessions_list_the_other_file_panels_cursor_counts() {
    let (_dir, mut app) = setup();
    app.right_panel = app.left_panel.clone();
    app.left_panel.kind = PanelKind::Sessions(Box::new(SessionsPanel::from_store(&app.sessions)));
    run(&mut app, None).expect("imported");
    let PanelKind::Sessions(panel) = &app.left_panel.kind else {
        panic!("still the sessions list");
    };
    assert_eq!(panel.session_count(), 1, "the list shows the import");
    run(&mut app, Some(PathBuf::from("demo.netbox"))).expect("relative to the other panel");
}

#[test]
fn without_a_path_or_a_netbox_file_under_the_cursor_there_is_nothing_to_read() {
    let (dir, mut app) = setup();
    fs::write(dir.path().join("notes.txt"), "x").expect("write");
    app.left_panel.reload().expect("reload");
    select(&mut app, "notes.txt");
    assert!(matches!(
        run(&mut app, None),
        Err(SessionError::NoImportSource)
    ));
    app.left_panel.kind = PanelKind::Tree(Box::default());
    app.right_panel.kind = PanelKind::Tree(Box::default());
    assert!(matches!(
        run(&mut app, None),
        Err(SessionError::NoImportSource)
    ));
    assert!(app.sessions.is_empty());
}

#[test]
fn an_unreadable_file_is_an_import_error_and_changes_nothing() {
    let (dir, mut app) = setup();
    let error = run(&mut app, Some(dir.path().join("missing.netbox"))).expect_err("missing");
    assert!(matches!(error, SessionError::Import(ImportError::Read(_))));
    assert!(app.sessions.is_empty());
}

#[test]
fn an_import_that_cannot_be_saved_is_not_taken_in() {
    let (dir, mut app) = setup();
    fs::write(dir.path().join(FILE_NAME), "{").expect("write");
    app.sessions = SessionStore::load_from(dir.path().join(FILE_NAME));
    assert!(matches!(
        run(&mut app, Some(dir.path().join("demo.netbox"))),
        Err(SessionError::Store(_))
    ));
    assert!(app.sessions.is_empty());
    assert!(
        transcript(&app).is_empty(),
        "nothing is reported as imported"
    );
}

fn large_export() -> String {
    use std::fmt::Write;

    let mut text = String::from("<NetBox><Sessions>");
    let padding = "x".repeat(450);
    for index in 0..1000 {
        write!(
            text,
            "<Session name=\"{index:04}{padding}\"><HostName>h.test</HostName>\
             <PublicKeyFile>{padding}</PublicKeyFile></Session>"
        )
        .expect("write XML");
    }
    text.push_str("</Sessions></NetBox>");
    text
}

#[test]
fn an_import_exceeding_the_store_limit_preserves_committed_sessions() {
    let (dir, mut app) = setup();
    app.sessions = file_store(dir.path(), vec![session("kept.test", None)]);
    let previous = app.sessions.clone();
    let saved_path = dir.path().join(FILE_NAME);
    let saved_bytes = fs::read(&saved_path).expect("previous file");
    let export = large_export();
    assert!(u64::try_from(export.len()).expect("fits") < MAX_FILE_BYTES);
    let path = dir.path().join("large.netbox");
    fs::write(&path, export).expect("write export");

    assert!(matches!(
        run(&mut app, Some(path)),
        Err(SessionError::Store(StoreError::TooLarge {
            limit: MAX_FILE_BYTES
        }))
    ));

    assert_eq!(app.sessions, previous);
    assert_eq!(fs::read(saved_path).expect("kept file"), saved_bytes);
    assert_eq!(
        SessionStore::load_from(dir.path().join(FILE_NAME)),
        previous
    );
    assert!(transcript(&app).is_empty(), "no successful import report");
}

#[test]
fn an_import_keeps_the_users_own_session_of_the_same_name() {
    let (dir, mut app) = setup();
    let mut mine = session("mine.test", None);
    mine.name = name("web");
    app.sessions = file_store(dir.path(), vec![mine]);
    run(&mut app, None).expect("imported");
    assert_eq!(
        app.sessions.find(&name("web")).map(|s| s.host.to_string()),
        Some("mine.test".to_owned())
    );
    assert_eq!(app.status_text(), "NetBox: 0 imported, 2 skipped");
}

#[test]
fn the_prompt_offers_the_netbox_file_under_the_cursor_or_nothing() {
    let (dir, mut app) = setup();
    prompt(&mut app);
    let Overlay::InputPrompt(asked) = &app.overlay else {
        panic!("the input prompt");
    };
    assert_eq!(asked.purpose(), &InputPurpose::ImportNetBox);
    assert_eq!(
        asked.input().text(),
        dir.path().join("demo.netbox").display().to_string()
    );

    select(&mut app, "..");
    prompt(&mut app);
    let Overlay::InputPrompt(asked) = &app.overlay else {
        panic!("the input prompt");
    };
    assert_eq!(asked.input().text(), "");
}

#[test]
fn the_prompts_answer_imports_or_reports_why_not() {
    let (dir, mut app) = setup();
    submit(
        &mut app,
        &format!("  {}  ", dir.path().join("demo.netbox").display()),
    );
    assert_eq!(app.sessions.len(), 1);
    select(&mut app, "..");
    submit(&mut app, "   ");
    assert!(
        app.status_text()
            .starts_with("Error: NetBox: name a .netbox file"),
        "{}",
        app.status_text()
    );
}

/// With no file list on either side, a relative path is taken from the
/// active panel's own directory.
#[test]
fn with_no_file_panel_a_relative_path_is_taken_from_the_active_panel() {
    let (_dir, mut app) = setup();
    app.left_panel.kind = PanelKind::Sessions(Box::new(SessionsPanel::from_store(&app.sessions)));
    app.right_panel.kind = PanelKind::Tree(Box::default());
    run(&mut app, Some(PathBuf::from("demo.netbox"))).expect("imported");
    assert_eq!(app.sessions.len(), 1);
}

/// With no saved password in the file there is nothing to say about one.
#[test]
fn an_import_without_passwords_says_nothing_about_them() {
    let (dir, mut app) = setup();
    fs::write(
        dir.path().join("plain.netbox"),
        "<NetBox><Sessions><Session><HostName>h.test</HostName></Session></Sessions></NetBox>",
    )
    .expect("write");
    run(&mut app, Some(dir.path().join("plain.netbox"))).expect("imported");
    assert!(transcript(&app).is_empty(), "{:?}", transcript(&app));
    assert_eq!(app.status_text(), "NetBox: 1 imported, 0 skipped");
}
