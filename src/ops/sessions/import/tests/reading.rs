//! Purpose: Which `.netbox` file is read, and the prompt that asks for one.

use super::*;

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
