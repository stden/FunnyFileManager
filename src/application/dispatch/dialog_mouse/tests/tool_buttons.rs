//! Purpose: the button maps of the tool dialogs — the vault's, Extract's and Install MCP server's — via `dialog_button_key`.

use super::*;

#[cfg(feature = "keepass")]
#[test]
fn locked_vault_buttons_use_unlock_and_cancel_keys() {
    let dir = TempDir::new().expect("temp");
    let mut app = app_at(&dir);
    let session = crate::kdbx::KdbxSession::new(dir.path().join("vault.kdbx"));
    app.set_overlay(Overlay::Kdbx(crate::models::KdbxOverlay(session)));
    assert_eq!(code(&mut app, 0), KeyCode::Enter);
    let reveal = dialog_button_key(&mut app, 1).expect("reveal button");
    assert_eq!(reveal.code, KeyCode::Char('r'));
    assert_eq!(reveal.modifiers, KeyModifiers::CONTROL);
    assert_eq!(code(&mut app, 2), KeyCode::Esc);
    assert_eq!(dialog_button_key(&mut app, 3), None);
}

#[cfg(feature = "keepass")]
#[test]
fn unlocked_vault_does_not_activate_stale_unlock_buttons() {
    let (_dir, session, _now) = crate::kdbx::tests_support::unlocked_sample();
    assert_eq!(kdbx_button_key(&session, 0), None);
    assert_eq!(kdbx_button_key(&session, 1), None);
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
