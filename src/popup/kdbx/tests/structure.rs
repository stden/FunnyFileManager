use super::*;
use crate::kdbx::vault::{BrowsePage, Page};

#[test]
fn tree_is_default_and_navigation_moves_between_entries() {
    let (_temp, mut app) = app_with_unlocked_vault();
    let page = app.kdbx_session().unwrap().vault().unwrap().page;
    assert_eq!(page, Page::Browse(BrowsePage::Tree));

    process_popup_input(&mut app, key(KeyCode::Down));
    assert_eq!(app.kdbx_session().unwrap().vault().unwrap().selected(), 1);
    process_popup_input(&mut app, key(KeyCode::End));
    assert_eq!(app.kdbx_session().unwrap().vault().unwrap().selected(), 4);
    process_popup_input(&mut app, key(KeyCode::Home));
    assert_eq!(app.kdbx_session().unwrap().vault().unwrap().selected(), 0);

    process_popup_input(&mut app, key(KeyCode::F(6)));
    assert_eq!(
        app.kdbx_session().unwrap().vault().unwrap().page,
        Page::Browse(BrowsePage::Entries)
    );
    process_popup_input(&mut app, key(KeyCode::F(6)));
    assert_eq!(
        app.kdbx_session().unwrap().vault().unwrap().page,
        Page::Browse(BrowsePage::Tree)
    );
}

#[test]
fn an_unchanged_form_closes_and_a_failed_save_retains_the_edited_form() {
    let (_temp, mut app) = app_with_unlocked_vault();
    process_popup_input(&mut app, key(KeyCode::F(4)));
    process_popup_input(&mut app, key(KeyCode::Esc));
    assert!(app
        .kdbx_session()
        .expect("session")
        .vault()
        .expect("vault")
        .form()
        .is_none());
    process_popup_input(&mut app, key(KeyCode::F(4)));
    type_str(&mut app, " changed");
    let path = app.kdbx_session().expect("session").path.clone();
    std::fs::remove_file(path).expect("remove fixture vault");
    process_popup_input(&mut app, key(KeyCode::F(2)));
    let form = app
        .kdbx_session()
        .expect("session")
        .vault()
        .expect("vault")
        .form()
        .expect("draft retained");
    assert!(form.error().is_some());
    assert!(form.title().ends_with(" changed"));
}

#[test]
fn probe_f6_while_locked_is_swallowed_without_a_vault() {
    let (_dir, path) = vault_file(SAMPLE, VAULT_PASSWORD, None);
    let mut app = App::new().expect("test app");
    app.set_overlay(Overlay::Kdbx(KdbxOverlay(KdbxSession::new(path))));

    assert!(process_popup_input(&mut app, key(KeyCode::F(6))));

    let session = app.kdbx_session().expect("overlay still up");
    assert!(session.state.is_locked());
    assert!(session.vault().is_none());
}
