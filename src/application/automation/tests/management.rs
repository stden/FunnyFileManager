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

#[test]
fn netbox_and_vault_executors_are_noops_outside_their_models() {
    let mut fixture = Fixture::new();
    execute(
        &mut fixture.terminal,
        &mut fixture.app,
        Invocation::NetboxPanel(NetboxCommand::NewSession),
    );
    execute(
        &mut fixture.terminal,
        &mut fixture.app,
        Invocation::Vault(VaultCommand::Close),
    );
    assert!(matches!(fixture.app.overlay, Overlay::None));
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

fn open_database_explorer(fixture: &mut Fixture) {
    use crate::models::database::{ConnectionSpec, DatabaseExplorer, Driver, Name};
    let spec = ConnectionSpec {
        name: Name::new("local").unwrap(),
        driver: Driver::Sqlite {
            path: fixture.root.path().join("local.db"),
        },
    };
    fixture.app.overlay = Overlay::Database(Box::new(DatabaseExplorer::new(vec![spec], None)));
}

#[test]
fn database_explorer_commands_reach_the_open_explorer() {
    use crate::action::database::DatabaseCommand;
    use crate::models::database::Task;
    let mut fixture = Fixture::new();
    open_database_explorer(&mut fixture);
    execute(
        &mut fixture.terminal,
        &mut fixture.app,
        Invocation::DatabaseExplorer(DatabaseCommand::Expand),
    );
    let request = fixture.app.database_requests.take().unwrap();
    assert!(matches!(request.task, Task::Schemas(spec) if spec.name.as_str() == "local"));
    assert!(matches!(fixture.app.overlay, Overlay::Database(_)));
    execute(
        &mut fixture.terminal,
        &mut fixture.app,
        Invocation::DatabaseExplorer(DatabaseCommand::Close),
    );
    assert!(matches!(fixture.app.overlay, Overlay::None));
}

#[test]
fn database_explorer_invocations_are_accepted_while_it_is_open() {
    use crate::action::database::DatabaseCommand;
    let mut fixture = Fixture::new();
    open_database_explorer(&mut fixture);
    let result = invoke(
        &mut fixture.terminal,
        &mut fixture.app,
        Invocation::DatabaseExplorer(DatabaseCommand::Close),
    );
    assert!(
        result.is_ok(),
        "the open explorer must accept its own commands: {result:?}"
    );
    assert!(matches!(fixture.app.overlay, Overlay::None));
}
