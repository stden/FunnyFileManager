//! Purpose: The session dialog, copies and deletes, and changes the file does not take.

use super::*;

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
