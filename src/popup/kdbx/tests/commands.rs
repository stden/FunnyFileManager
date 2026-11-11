//! Purpose: Named vault commands preserve modal behavior; direct guards reject stray calls.

use super::*;
use crate::action::vault::VaultCommand;
use crate::automation::catalog::Invocation;
use crate::kdbx::vault::Page;
use crate::test_support::WritableTestBackend;
use ratatui::Terminal;

fn named(app: &mut App, command: VaultCommand) {
    let mut terminal = Terminal::new(WritableTestBackend::new(80, 24)).expect("terminal");
    crate::application::automation::invoke(&mut terminal, app, Invocation::Vault(command))
        .expect("named vault invocation");
}

#[test]
fn typed_vault_close_preserves_an_open_edit_form() {
    let (_dir, mut app) = app_with_unlocked_vault();
    super::super::apply(&mut app, VaultCommand::Edit);
    super::super::apply(&mut app, VaultCommand::Close);
    assert!(app
        .kdbx_session()
        .and_then(KdbxSession::vault)
        .and_then(crate::kdbx::Vault::form)
        .is_some());
}

#[test]
fn typed_vault_commands_preserve_the_delete_question() {
    let (_dir, mut app) = app_with_unlocked_vault();
    super::super::apply(&mut app, VaultCommand::Delete);
    super::super::apply(&mut app, VaultCommand::ToggleView);
    assert!(matches!(
        app.kdbx_session()
            .and_then(KdbxSession::vault)
            .expect("vault")
            .page,
        Page::DeleteConfirm(_)
    ));
    super::super::apply(&mut app, VaultCommand::Lock);
    assert!(matches!(app.overlay, Overlay::None));
}

#[test]
fn typed_vault_commands_without_a_session_leave_application_running() {
    let mut app = App::new().expect("app");
    let status = app.status_text();
    super::super::apply(&mut app, VaultCommand::LockAndQuit);
    super::super::remember_and_close(&mut app);
    assert!(app.lifecycle.is_running());
    assert_eq!(app.status_text(), status);

    let (_dir, mut unlocked) = app_with_unlocked_vault();
    super::super::apply(&mut unlocked, VaultCommand::Save);
    super::super::browse::browse_command(
        &mut unlocked,
        VaultCommand::Lock,
        std::time::Instant::now(),
    );
    assert!(matches!(unlocked.overlay, Overlay::None));
}

#[test]
fn named_vault_save_writes_the_entry_form() {
    let (_dir, mut app) = app_with_unlocked_vault();
    focus(&mut app, "GitHub");
    named(&mut app, VaultCommand::Edit);
    for _ in 0..4 {
        process_popup_input(&mut app, key(KeyCode::Tab));
    }
    type_str(&mut app, "-rotated");
    named(&mut app, VaultCommand::Save);
    let session = app.kdbx_session_mut().expect("session");
    assert!(session.vault().expect("vault").form().is_none());
    assert!(session
        .copy_password(std::time::Instant::now())
        .expect("password")
        .ends_with("-rotated"));
}

#[test]
fn named_vault_undo_reverts_the_latest_form_keystroke() {
    let (_dir, mut app) = app_with_unlocked_vault();
    focus(&mut app, "GitHub");
    named(&mut app, VaultCommand::Edit);
    type_str(&mut app, "X");
    named(&mut app, VaultCommand::Undo);
    let form = app
        .kdbx_session()
        .and_then(KdbxSession::vault)
        .and_then(crate::kdbx::Vault::form)
        .expect("form");
    assert_eq!(form.title(), "GitHub");
}

#[test]
fn named_vault_reveal_closes_the_password_dialog_and_hides_the_password() {
    let (_dir, mut app) = app_with_unlocked_vault();
    focus(&mut app, "GitHub");
    named(&mut app, VaultCommand::Reveal);
    assert!(matches!(
        app.kdbx_session()
            .and_then(KdbxSession::vault)
            .expect("vault")
            .page,
        Page::Password(_)
    ));
    named(&mut app, VaultCommand::Reveal);
    let vault = app
        .kdbx_session()
        .and_then(KdbxSession::vault)
        .expect("vault");
    assert!(matches!(vault.page, Page::Browse(_)));
    assert!(!vault.is_revealed());
}

#[test]
fn browse_save_keeps_the_vault_open_and_browse_lock_zeroes_and_closes_it() {
    let (_dir, mut app) = app_with_unlocked_vault();
    let now = std::time::Instant::now();
    super::super::browse::browse_command(&mut app, VaultCommand::Save, now);
    assert!(app.kdbx_session().is_some());
    super::super::browse::browse_command(&mut app, VaultCommand::Lock, now);
    assert!(matches!(app.overlay, Overlay::None));
    assert!(matches!(app.status, crate::status::Status::VaultLocked));
}
