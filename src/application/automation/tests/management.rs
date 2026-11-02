//! Purpose: Named management actions retain their real requests and nested dialog boundaries.

use super::*;
use crate::keymap::Context;
use crate::models::{
    lsp_editor::{Access, Mode, ReadOnlyReason},
    LspEditor, WslDialog,
};

#[test]
fn wsl_refresh_queues_a_request_and_close_leaves_the_dialog() {
    let mut fixture = Fixture::new();
    fixture.app.overlay = Overlay::Wsl(Box::new(WslDialog::new(Vec::new())));
    assert_eq!(current_context(&fixture.app), Some(Context::WslManager));
    fixture.run("wsl_manager", "refresh").unwrap();
    let Overlay::Wsl(dialog) = &fixture.app.overlay else {
        panic!("WSL closed")
    };
    assert!(matches!(
        dialog.activity,
        crate::models::WslActivity::Busy { .. }
    ));
    fixture.run("wsl_manager", "close").unwrap();
    assert!(matches!(fixture.app.overlay, Overlay::None));
}

#[test]
fn language_server_view_is_named_but_its_exit_question_owns_input() {
    let mut fixture = Fixture::new();
    fixture.app.overlay = Overlay::LspEditor(Box::new(LspEditor::new(
        Access::ReadOnly(ReadOnlyReason::NoSettingsFile),
        crate::config::LspSettings::default(),
    )));
    assert_eq!(
        current_context(&fixture.app),
        Some(Context::LanguageServers)
    );
    fixture.run("language_servers", "view").unwrap();
    let Overlay::LspEditor(editor) = &mut fixture.app.overlay else {
        panic!("LSP closed")
    };
    assert!(matches!(editor.mode(), Mode::View(_)));
    editor.set_mode(Mode::ConfirmExit(crate::models::ExitChoice::Cancel));
    assert_eq!(current_context(&fixture.app), None);
    assert!(fixture.run("language_servers", "close").is_err());
}

#[test]
fn path_add_opens_the_real_form_and_prevents_browse_actions_behind_it() {
    let mut fixture = Fixture::new();
    let editor = crate::path_env::PathEditor::new(
        crate::path_env::Scope::User,
        crate::path_env::Snapshot {
            value: String::new(),
            revision: String::new(),
            location: String::new(),
        },
    );
    fixture.app.overlay = Overlay::PathEditor(Box::new(editor));
    assert_eq!(current_context(&fixture.app), Some(Context::PathEditor));
    fixture.run("path_editor", "add").unwrap();
    let Overlay::PathEditor(editor) = &fixture.app.overlay else {
        panic!("PATH closed")
    };
    assert!(matches!(editor.mode, crate::path_env::Mode::Edit { .. }));
    assert_eq!(current_context(&fixture.app), None);
    assert!(fixture.run("path_editor", "remove").is_err());
}

#[cfg(feature = "keepass")]
#[test]
fn vault_close_uses_its_own_context_without_opening_a_database() {
    let mut fixture = Fixture::new();
    fixture.app.overlay = Overlay::Kdbx(crate::models::KdbxOverlay(crate::kdbx::KdbxSession::new(
        fixture.root.path().join("missing.kdbx"),
    )));
    assert_eq!(current_context(&fixture.app), Some(Context::Vault));
    fixture.run("vault", "close").unwrap();
    assert!(matches!(fixture.app.overlay, Overlay::None));
}
