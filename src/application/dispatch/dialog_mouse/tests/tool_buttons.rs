//! Purpose: the button maps of the tool dialogs — the vault's, Extract's and Install MCP server's — via `dialog_button_key`.

use super::*;

/// The unlock prompt's buttons are the vault's own commands — Unlock,
/// Reveal, Cancel — not keys of a dialog protocol.
#[cfg(feature = "keepass")]
#[test]
fn locked_vault_buttons_are_unlock_reveal_and_cancel() {
    use crate::action::vault::VaultCommand;
    let dir = TempDir::new().expect("temp");
    let mut app = app_at(&dir);
    let session = crate::kdbx::KdbxSession::new(dir.path().join("vault.kdbx"));
    app.set_overlay(Overlay::Kdbx(crate::models::KdbxOverlay(session)));
    assert_eq!(vault_prompt_button(&app, 0), Some(VaultCommand::Activate));
    assert_eq!(vault_prompt_button(&app, 1), Some(VaultCommand::Reveal));
    assert_eq!(vault_prompt_button(&app, 2), Some(VaultCommand::Close));
    assert_eq!(vault_prompt_button(&app, 3), None);
    assert_eq!(dialog_button_key(&mut app, 0), None, "no protocol key");
}

/// Through the pointer: a click on the drawn Reveal button shows the typed
/// master password, as Ctrl+R does.
#[cfg(feature = "keepass")]
#[test]
fn clicking_the_reveal_button_shows_the_master_password() {
    let dir = TempDir::new().expect("temp");
    let mut app = app_at(&dir);
    let mut term = terminal();
    let session = crate::kdbx::KdbxSession::new(dir.path().join("vault.kdbx"));
    app.set_overlay(Overlay::Kdbx(crate::models::KdbxOverlay(session)));
    popup::process_popup_input(&mut app, KeyEvent::from(KeyCode::Char('s')));
    draw(&mut app, &mut term);
    let &(_, left, _) = app
        .dialog_mouse
        .buttons
        .iter()
        .find(|(button, _, _)| *button == 1)
        .expect("the Reveal button was drawn");
    let buttons_y = app.dialog_mouse.buttons_y;

    dispatch_dialog_mouse(
        &mut term,
        &mut app,
        mouse(MouseEventKind::Down(MouseButton::Left), left, buttons_y),
    );

    let prompt = app
        .kdbx_session()
        .and_then(crate::kdbx::KdbxSession::prompt);
    assert!(prompt.is_some_and(crate::kdbx::UnlockPrompt::is_revealed));
}

#[cfg(feature = "keepass")]
#[test]
fn unlocked_vault_does_not_activate_stale_unlock_buttons() {
    let (dir, session, _now) = crate::kdbx::tests_support::unlocked_sample();
    assert_eq!(kdbx_button_key(&session, 0), None);
    assert_eq!(kdbx_button_key(&session, 1), None);
    let mut app = app_at(&dir);
    app.set_overlay(Overlay::Kdbx(crate::models::KdbxOverlay(session)));
    assert_eq!(vault_prompt_button(&app, 0), None, "no prompt is up");
}

/// The unlocked vault's own dialogs (password, delete confirmation) answer
/// OK with Enter and every other button with Esc.
#[cfg(feature = "keepass")]
#[test]
fn unlocked_vault_dialog_buttons_confirm_or_cancel() {
    let (_dir, mut session, now) = crate::kdbx::tests_support::unlocked_sample();
    let codes = |session: &crate::kdbx::KdbxSession| {
        [0, 1, 2].map(|index| kdbx_button_key(session, index).map(|key| key.code))
    };
    let confirm_or_cancel = [Some(KeyCode::Enter), Some(KeyCode::Esc), Some(KeyCode::Esc)];

    session.reveal(now).expect("password dialog");
    assert_eq!(codes(&session), confirm_or_cancel);

    session.close_vault_dialog(now);
    session.request_delete(now).expect("delete confirmation");
    assert_eq!(codes(&session), confirm_or_cancel);
}

#[test]
fn zz_probe_extract_buttons_name_extract_and_cancel_before_enter() {
    use crate::models::{ExtractField, ExtractPrompt};
    use std::path::PathBuf;

    let dir = TempDir::new().expect("temp");
    let mut app = app_at(&dir);
    app.set_overlay(Overlay::ExtractPrompt(Box::new(ExtractPrompt::new(
        vec![PathBuf::from("/src/a.tar.gz")],
        Path::new("/dest"),
    ))));

    assert_eq!(code(&mut app, 1), KeyCode::Enter);
    let Overlay::ExtractPrompt(prompt) = &app.overlay else {
        panic!("extract prompt");
    };
    assert_eq!(prompt.field, ExtractField::Cancel);

    assert_eq!(code(&mut app, 0), KeyCode::Enter);
    let Overlay::ExtractPrompt(prompt) = &app.overlay else {
        panic!("extract prompt");
    };
    assert_eq!(prompt.field, ExtractField::Extract);
}

/// Register / Cancel of the Install MCP server dialog name the button before
/// Enter; a click on a dimmed Register — no server to register — does nothing.
#[test]
fn mcp_install_buttons_name_register_and_cancel_before_enter() {
    use crate::mcp_clients::{Scope, ServerBinary, ServerState, Survey};
    use crate::models::{McpInstallDialog, McpInstallField};

    let dir = TempDir::new().expect("temp");
    let mut app = app_at(&dir);
    let survey = |server| Survey {
        server,
        folder: None,
        scope: Scope::AllProjects,
        clients: Vec::new(),
    };
    let found = ServerState::Found(ServerBinary::new(dir.path().join("ffm-mcp")).expect("unicode"));
    app.set_overlay(Overlay::McpInstall(Box::new(McpInstallDialog::new(
        survey(found),
    ))));
    let field = |app: &App| match &app.overlay {
        Overlay::McpInstall(dialog) => dialog.field,
        _ => panic!("the MCP dialog"),
    };
    assert_eq!(code(&mut app, 0), KeyCode::Enter);
    assert_eq!(field(&app), McpInstallField::Register);
    assert_eq!(code(&mut app, 1), KeyCode::Enter);
    assert_eq!(field(&app), McpInstallField::Cancel);

    let missing = McpInstallDialog::new(survey(ServerState::Missing));
    app.set_overlay(Overlay::McpInstall(Box::new(missing)));
    assert_eq!(dialog_button_key(&mut app, 0), None);
    assert_eq!(field(&app), McpInstallField::Scope);
}
