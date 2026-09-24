//! Purpose: unlocking the vault and leaving it — the master password, Esc remembering the session, Ctrl+C locking on the way out.

use super::*;

#[test]
fn ctrl_r_toggles_the_typed_master_password() {
    let (_dir, path) = vault_file(SAMPLE, VAULT_PASSWORD, None);
    let mut app = App::new().expect("test app");
    app.set_overlay(Overlay::Kdbx(KdbxOverlay(KdbxSession::new(path))));
    type_str(&mut app, "secret");

    process_popup_input(&mut app, ctrl('r'));

    assert!(app.kdbx_session().unwrap().prompt().unwrap().is_revealed());
}

#[test]
fn enter_unlocks_with_the_right_password() {
    let (_dir, path) = vault_file(SAMPLE, VAULT_PASSWORD, None);
    let mut app = App::new().expect("test app");
    app.set_overlay(Overlay::Kdbx(KdbxOverlay(KdbxSession::new(path))));

    type_str(&mut app, VAULT_PASSWORD);
    process_popup_input(&mut app, key(KeyCode::Enter));

    assert!(app
        .kdbx_session()
        .is_some_and(|session| !session.state.is_locked()));
    assert!(
        app.status_text().contains("Vault unlocked: 5 entries"),
        "{}",
        app.status_text()
    );
}

#[test]
fn a_wrong_password_reports_failure_and_stays_locked() {
    let (_dir, path) = vault_file(SAMPLE, VAULT_PASSWORD, None);
    let mut app = App::new().expect("test app");
    app.set_overlay(Overlay::Kdbx(KdbxOverlay(KdbxSession::new(path))));

    type_str(&mut app, "not the password");
    process_popup_input(&mut app, key(KeyCode::Enter));

    let session = app.kdbx_session().expect("overlay still up");
    assert!(session.state.is_locked());
    // The status line carries no detail; the reason sits beside the field.
    assert!(
        app.status_text().contains("Unlock failed"),
        "{}",
        app.status_text()
    );
    assert!(session.prompt().expect("locked").error().is_some());
}

#[test]
fn esc_remembers_and_closes_the_overlay() {
    let (_dir, mut app) = app_with_unlocked_vault();

    let consumed = process_popup_input(&mut app, key(KeyCode::Esc));

    assert!(consumed);
    assert!(matches!(app.overlay, Overlay::None));
    assert!(
        app.status_text().contains("key remembered"),
        "{}",
        app.status_text()
    );
}

#[test]
fn ctrl_c_still_quits_but_locks_on_the_way_out() {
    let (_dir, mut app) = app_with_unlocked_vault();

    process_popup_input(&mut app, ctrl('c'));

    assert!(!app.lifecycle.is_running());
    assert!(
        matches!(app.overlay, Overlay::None),
        "the session must be locked and dropped at the quit gesture"
    );
}
