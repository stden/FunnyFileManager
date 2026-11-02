//! Purpose: The sessions panel: opening, a damaged store, going back to files, refresh and `:sessions`.

use super::*;

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
    assert_eq!(status, Status::Error(DAMAGED.to_owned()));
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
