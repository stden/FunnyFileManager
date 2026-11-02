//! Purpose: F8 in the vault deletes the selected entry after its own confirmation, never a panel file, and Ctrl+Z brings it back.

use super::*;

#[test]
fn kdbx_overlay_swallows_f8_without_touching_the_panel() {
    let (_vault_dir, mut app) = app_with_unlocked_vault();
    let panel_dir = TempDir::new().expect("temp dir");
    File::create(panel_dir.path().join("precious.txt")).expect("create file");
    app.left_panel = Panel::new(panel_dir.path().to_path_buf()).expect("panel");
    app.left_panel.select(Some(1));

    let consumed = process_popup_input(&mut app, key(KeyCode::F(8)));

    assert!(consumed);
    assert!(app.kdbx_session().is_some(), "the overlay must stay up");
    assert!(matches!(
        app.kdbx_session().unwrap().vault().unwrap().page,
        crate::kdbx::vault::Page::DeleteConfirm(_)
    ));
    assert!(
        panel_dir.path().join("precious.txt").exists(),
        "F8 must not reach the delete handler"
    );
}

#[test]
fn f8_deletes_the_selected_entry_and_ctrl_z_restores_it() {
    let (_dir, mut app) = app_with_unlocked_vault();
    focus(&mut app, "GitHub");

    process_popup_input(&mut app, key(KeyCode::F(8)));
    process_popup_input(&mut app, key(KeyCode::Enter));
    assert_eq!(
        app.kdbx_session().unwrap().vault().unwrap().entries().len(),
        4
    );
    assert!(app.status_text().contains("KeePass entry deleted"));

    process_popup_input(&mut app, ctrl('z'));
    assert_eq!(
        app.kdbx_session().unwrap().vault().unwrap().entries().len(),
        5
    );
    assert!(app.status_text().contains("undone"));
}

#[test]
fn zzz_probe_delete_cancel_sets_status() {
    let (_dir, mut app) = app_with_unlocked_vault();
    focus(&mut app, "GitHub");
    process_popup_input(&mut app, key(KeyCode::F(8)));
    assert!(matches!(
        app.kdbx_session().unwrap().vault().unwrap().page,
        crate::kdbx::vault::Page::DeleteConfirm(_)
    ));

    let consumed = process_popup_input(&mut app, key(KeyCode::Char('n')));

    assert!(consumed);
    assert!(matches!(
        app.kdbx_session().unwrap().vault().unwrap().page,
        crate::kdbx::vault::Page::Browse(_)
    ));
    assert_eq!(
        app.kdbx_session().unwrap().vault().unwrap().entries().len(),
        5
    );
    assert!(
        app.status_text().contains("Delete cancelled"),
        "status was {:?}",
        app.status_text()
    );
}

#[test]
fn zzz_probe_delete_cancel_via_esc_sets_status() {
    let (_dir, mut app) = app_with_unlocked_vault();
    focus(&mut app, "GitHub");
    process_popup_input(&mut app, key(KeyCode::F(8)));

    process_popup_input(&mut app, key(KeyCode::Esc));

    assert!(app.kdbx_session().is_some(), "overlay must stay up");
    assert!(app.status_text().contains("Delete cancelled"));
}
