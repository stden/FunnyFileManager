//! Purpose: the vault's F4 edit form through the popup — F2 saves, Ctrl+Z undoes a keystroke, Ctrl+C copies its password, and a dirty form asks before it closes.

use super::*;

#[test]
fn f4_edits_and_f2_saves_the_selected_entry_through_the_popup() {
    let (_dir, mut app) = app_with_unlocked_vault();
    focus(&mut app, "GitHub");
    process_popup_input(&mut app, key(KeyCode::F(4)));
    assert!(app
        .kdbx_session()
        .and_then(KdbxSession::vault)
        .and_then(crate::kdbx::Vault::form)
        .is_some());
    for _ in 0..4 {
        process_popup_input(&mut app, key(KeyCode::Tab));
    }
    type_str(&mut app, "-rotated");
    process_popup_input(&mut app, key(KeyCode::F(2)));
    let session = app.kdbx_session_mut().expect("session");
    assert!(session.vault().expect("vault").form().is_none());
    assert!(session
        .copy_password(std::time::Instant::now())
        .expect("password")
        .ends_with("-rotated"));
}

#[test]
fn ctrl_c_copies_the_visible_password_from_the_edit_form() {
    let (_dir, mut app) = app_with_unlocked_vault();
    let recorder = RecordingClipboard::default();
    app.clipboard.install(Box::new(recorder.clone()));
    focus(&mut app, "GitHub");
    process_popup_input(&mut app, key(KeyCode::F(4)));

    process_popup_input(&mut app, ctrl('c'));

    assert_eq!(recorder.0.borrow().as_slice(), [password_for("GitHub")]);
    assert!(app
        .kdbx_session()
        .unwrap()
        .vault()
        .unwrap()
        .form()
        .is_some());
}

#[test]
fn ctrl_z_undoes_the_latest_edit_keystroke() {
    let (_dir, mut app) = app_with_unlocked_vault();
    focus(&mut app, "GitHub");
    process_popup_input(&mut app, key(KeyCode::F(4)));
    type_str(&mut app, "X");

    process_popup_input(&mut app, ctrl('z'));

    assert_eq!(
        app.kdbx_session()
            .unwrap()
            .vault()
            .unwrap()
            .form()
            .unwrap()
            .title(),
        "GitHub"
    );
}

#[test]
fn ctrl_z_with_another_modifier_held_is_not_the_form_undo() {
    let ctrl = KeyModifiers::CONTROL;
    for modifiers in [ctrl | KeyModifiers::SHIFT, ctrl | KeyModifiers::ALT] {
        let (_dir, mut app) = app_with_unlocked_vault();
        focus(&mut app, "GitHub");
        process_popup_input(&mut app, key(KeyCode::F(4)));
        type_str(&mut app, "X");

        process_popup_input(&mut app, KeyEvent::new(KeyCode::Char('z'), modifiers));

        let form = app.kdbx_session().unwrap().vault().unwrap().form().unwrap();
        assert_eq!(form.title(), "GitHubX", "{modifiers:?} undid the keystroke");
    }
}

#[test]
fn dirty_form_asks_to_save_on_exit_and_n_discards() {
    let (_dir, mut app) = app_with_unlocked_vault();
    focus(&mut app, "GitHub");
    process_popup_input(&mut app, key(KeyCode::F(4)));
    type_str(&mut app, "X");

    process_popup_input(&mut app, key(KeyCode::Esc));
    assert_eq!(
        app.kdbx_session()
            .unwrap()
            .vault()
            .unwrap()
            .form()
            .unwrap()
            .mode(),
        crate::kdbx::FormMode::ConfirmSave
    );
    process_popup_input(&mut app, key(KeyCode::Char('n')));
    assert!(app
        .kdbx_session()
        .unwrap()
        .vault()
        .unwrap()
        .form()
        .is_none());
}

#[test]
fn enter_saves_when_a_dirty_form_asks_on_exit() {
    let (_dir, mut app) = app_with_unlocked_vault();
    focus(&mut app, "GitHub");
    process_popup_input(&mut app, key(KeyCode::F(4)));
    type_str(&mut app, "X");
    process_popup_input(&mut app, key(KeyCode::Esc));

    process_popup_input(&mut app, key(KeyCode::Enter));

    let vault = app.kdbx_session().unwrap().vault().unwrap();
    assert!(vault.form().is_none());
    assert!(vault.visible().any(|entry| entry.title == "GitHubX"));
}
