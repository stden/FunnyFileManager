//! Purpose: vault-browser failures surface as status and notice text, and stray keys change nothing.

use super::*;
use crate::kdbx::vault::{BrowsePage, Page};

fn vault(app: &App) -> &crate::kdbx::Vault {
    app.kdbx_session()
        .and_then(KdbxSession::vault)
        .expect("unlocked vault")
}

fn page(app: &App) -> Page {
    vault(app).page
}

fn notice(app: &App) -> String {
    vault(app).notice.clone().unwrap_or_default()
}

#[test]
fn ctrl_z_without_a_change_reports_nothing_to_undo() {
    let (_dir, mut app) = app_with_unlocked_vault();

    process_popup_input(&mut app, ctrl('z'));

    assert_eq!(app.status_text(), "Error: No KDBX change to undo");
    assert_eq!(notice(&app), app.status_text());
    assert_eq!(vault(&app).entries().len(), 5);
}

#[test]
fn ctrl_z_after_the_backup_disappeared_keeps_the_deletion_and_says_why() {
    let (_dir, mut app) = app_with_unlocked_vault();
    focus(&mut app, "GitHub");
    process_popup_input(&mut app, key(KeyCode::F(8)));
    process_popup_input(&mut app, key(KeyCode::Enter));
    let path = app.kdbx_session().expect("session").path.clone();
    std::fs::remove_file(format!("{}.bak", path.display())).expect("remove backup");

    process_popup_input(&mut app, ctrl('z'));

    assert!(
        app.status_text().contains("no KDBX change to undo"),
        "{}",
        app.status_text()
    );
    assert_eq!(vault(&app).entries().len(), 4, "the deletion stands");
}

#[test]
fn f8_with_a_filter_that_matches_nothing_reports_no_selection() {
    let (_dir, mut app) = app_with_unlocked_vault();
    type_str(&mut app, "no-such-entry");

    process_popup_input(&mut app, key(KeyCode::F(8)));

    assert_eq!(app.status_text(), "Error: No entry selected");
    assert_eq!(page(&app), Page::Browse(BrowsePage::Tree));
}

#[test]
fn f8_in_view_mode_refuses_before_asking_to_confirm() {
    let (_dir, mut app) = app_with_unlocked_vault();
    app.kdbx_session_mut().expect("session").access = crate::kdbx::Access::View;

    process_popup_input(&mut app, key(KeyCode::F(8)));

    assert!(
        app.status_text().starts_with("Error: View mode"),
        "{}",
        app.status_text()
    );
    assert_eq!(page(&app), Page::Browse(BrowsePage::Tree));
}

#[test]
fn a_confirmed_delete_of_a_vanished_vault_reports_the_failure_and_keeps_the_list() {
    let (_dir, mut app) = app_with_unlocked_vault();
    focus(&mut app, "GitHub");
    process_popup_input(&mut app, key(KeyCode::F(8)));
    let path = app.kdbx_session().expect("session").path.clone();
    std::fs::remove_file(path).expect("remove vault");

    process_popup_input(&mut app, key(KeyCode::Char('y')));

    assert!(
        app.status_text().starts_with("Delete failed: "),
        "{}",
        app.status_text()
    );
    assert_eq!(notice(&app), app.status_text());
    assert_eq!(vault(&app).entries().len(), 5);
    assert!(matches!(page(&app), Page::DeleteConfirm(_)));
}

#[test]
fn keys_without_a_meaning_leave_the_confirm_dialog_open() {
    let (_dir, mut app) = app_with_unlocked_vault();
    focus(&mut app, "GitHub");
    process_popup_input(&mut app, key(KeyCode::F(8)));

    assert!(process_popup_input(&mut app, key(KeyCode::Char('x'))));
    assert!(matches!(page(&app), Page::DeleteConfirm(_)));

    process_popup_input(&mut app, key(KeyCode::Char('n')));
    assert_eq!(page(&app), Page::Browse(BrowsePage::Tree));
    assert_eq!(vault(&app).entries().len(), 5);
}

#[test]
fn the_password_dialog_ignores_stray_keys_and_esc_closes_it() {
    let (_dir, mut app) = app_with_unlocked_vault();
    focus(&mut app, "GitHub");
    process_popup_input(&mut app, ctrl('r'));
    assert!(matches!(page(&app), Page::Password(_)));

    assert!(process_popup_input(&mut app, key(KeyCode::Char('x'))));
    assert!(matches!(page(&app), Page::Password(_)));
    assert!(vault(&app).is_revealed());

    process_popup_input(&mut app, key(KeyCode::Esc));
    assert!(matches!(page(&app), Page::Browse(_)));
    assert!(!vault(&app).is_revealed());
}

#[test]
fn unbound_browse_keys_are_swallowed_without_touching_the_vault() {
    let (_dir, mut app) = app_with_unlocked_vault();
    let before = vault(&app).selected();

    for event in [
        key(KeyCode::F(1)),
        key(KeyCode::Tab),
        KeyEvent::new(KeyCode::Char('x'), KeyModifiers::ALT),
    ] {
        assert!(process_popup_input(&mut app, event));
    }

    assert_eq!(vault(&app).selected(), before);
    assert_eq!(page(&app), Page::Browse(BrowsePage::Tree));
    assert!(app.kdbx_session().is_some(), "the overlay stays up");
}

#[test]
fn esc_on_a_locked_vault_closes_without_remembering_it() {
    let (_dir, path) = vault_file(SAMPLE, VAULT_PASSWORD, None);
    let mut app = App::new().expect("test app");
    app.set_overlay(Overlay::Kdbx(KdbxOverlay(KdbxSession::new(path))));

    process_popup_input(&mut app, key(KeyCode::Esc));

    assert!(matches!(app.overlay, Overlay::None));
    assert_eq!(
        app.status_text(),
        "Vault locked",
        "a locked vault must not claim its key was remembered"
    );
}

#[test]
fn the_edit_form_ignores_keys_it_has_no_use_for() {
    let (_dir, mut app) = app_with_unlocked_vault();
    focus(&mut app, "GitHub");
    process_popup_input(&mut app, key(KeyCode::F(4)));
    let form_title = |app: &App| vault(app).form().expect("form").title().to_owned();
    let title = form_title(&app);

    process_popup_input(&mut app, key(KeyCode::F(9)));
    assert_eq!(form_title(&app), title);

    type_str(&mut app, "!");
    process_popup_input(&mut app, key(KeyCode::Esc));
    let mode = |app: &App| vault(app).form().expect("form").mode();
    assert_eq!(mode(&app), crate::kdbx::FormMode::ConfirmSave);

    process_popup_input(&mut app, key(KeyCode::F(9)));
    assert_eq!(mode(&app), crate::kdbx::FormMode::ConfirmSave);
    assert_eq!(form_title(&app), format!("{title}!"));
}
