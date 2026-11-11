//! Unit tests for [`super`]: default marks, the focus ring, toggling, and a
//! scope switch resetting the marks.

use std::path::PathBuf;

use crate::mcp_clients::{Blocker, ServerBinary};

use super::*;

fn server() -> ServerState {
    ServerState::Found(ServerBinary::new(PathBuf::from("/srv/x/ffm-mcp")).unwrap())
}

fn status(client: ClientId, presence: Presence, state: ConfigState) -> ClientStatus {
    ClientStatus {
        client,
        presence,
        config: Some(PathBuf::from("/home/bob/config")),
        state,
    }
}

/// Claude Code not configured, Codex different, Junie not detected with no
/// config, Kiro registered.
fn survey(server: ServerState) -> Survey {
    Survey {
        server,
        folder: None,
        scope: Scope::AllProjects,
        clients: vec![
            status(
                ClientId::ClaudeCode,
                Presence::Detected,
                ConfigState::NotConfigured,
            ),
            status(
                ClientId::Codex,
                Presence::Detected,
                ConfigState::DifferentEntry,
            ),
            status(
                ClientId::Junie,
                Presence::NotDetected,
                ConfigState::NoConfig,
            ),
            status(ClientId::Kiro, Presence::Detected, ConfigState::Registered),
        ],
    }
}

fn marks(dialog: &McpInstallDialog) -> Vec<Mark> {
    dialog.rows.iter().map(|row| row.mark).collect()
}

#[test]
fn default_marks_follow_detection_and_state() {
    let dialog = McpInstallDialog::new(survey(server()));
    assert_eq!(
        marks(&dialog),
        [Mark::Checked, Mark::Checked, Mark::Unchecked, Mark::Checked]
    );
    assert_eq!(dialog.field, McpInstallField::Scope);
}

#[test]
fn a_blocked_row_starts_unchecked_and_disabled() {
    let mut survey = survey(server());
    survey.clients[0].state = ConfigState::Blocked(Blocker::NeedsClaudeCli);
    let dialog = McpInstallDialog::new(survey);
    assert_eq!(dialog.rows[0].mark, Mark::Unchecked);
    assert!(!dialog.is_enabled(&dialog.rows[0]));
}

#[test]
fn without_a_server_only_registered_rows_are_checked_and_nothing_is_enabled() {
    let dialog = McpInstallDialog::new(survey(ServerState::Missing));
    assert_eq!(
        marks(&dialog),
        [
            Mark::Unchecked,
            Mark::Unchecked,
            Mark::Unchecked,
            Mark::Checked
        ]
    );
    assert!(!dialog.can_register());
    assert!(dialog.rows.iter().all(|row| !dialog.is_enabled(row)));
    assert_eq!(
        dialog.focus_order(),
        [McpInstallField::Scope, McpInstallField::Cancel]
    );
}

#[test]
fn the_focus_ring_skips_registered_rows_and_wraps() {
    let mut dialog = McpInstallDialog::new(survey(server()));
    let expected = [
        McpInstallField::Scope,
        McpInstallField::Client(ClientId::ClaudeCode),
        McpInstallField::Client(ClientId::Codex),
        McpInstallField::Client(ClientId::Junie),
        McpInstallField::Register,
        McpInstallField::Cancel,
    ];
    assert_eq!(dialog.focus_order(), expected);
    for field in expected.iter().skip(1).chain([&McpInstallField::Scope]) {
        dialog.advance(Direction::Forward);
        assert_eq!(dialog.field, *field);
    }
    dialog.advance(Direction::Backward);
    assert_eq!(dialog.field, McpInstallField::Cancel);
}

#[test]
fn space_toggles_only_an_enabled_client_row() {
    let mut dialog = McpInstallDialog::new(survey(server()));
    dialog.toggle();
    assert_eq!(marks(&dialog)[0], Mark::Checked, "the radio has no mark");
    dialog.field = McpInstallField::Client(ClientId::Junie);
    dialog.toggle();
    assert_eq!(dialog.rows[2].mark, Mark::Checked);
    dialog.toggle();
    assert_eq!(dialog.rows[2].mark, Mark::Unchecked);
    dialog.field = McpInstallField::Client(ClientId::Kiro);
    dialog.toggle();
    assert_eq!(
        dialog.rows[3].mark,
        Mark::Checked,
        "a registered row stays done"
    );
}

#[test]
fn checked_clients_include_the_registered_ones_in_table_order() {
    let dialog = McpInstallDialog::new(survey(server()));
    assert_eq!(
        dialog.checked_clients(),
        [ClientId::ClaudeCode, ClientId::Codex, ClientId::Kiro]
    );
}

#[test]
fn applying_a_survey_resets_marks_and_keeps_a_valid_focus() {
    let mut dialog = McpInstallDialog::new(survey(server()));
    dialog.rows[0].mark = Mark::Unchecked;
    dialog.field = McpInstallField::Register;
    let mut folder = survey(server());
    folder.scope = Scope::CurrentFolder;
    dialog.apply(folder);
    assert_eq!(dialog.rows[0].mark, Mark::Checked);
    assert_eq!(dialog.field, McpInstallField::Register);
    assert_eq!(dialog.scope_index(), 1);
    dialog.apply(survey(ServerState::Missing));
    assert_eq!(dialog.field, McpInstallField::Scope, "Register went away");
    assert_eq!(dialog.scope_index(), 0);
}

#[test]
fn a_mark_toggles_both_ways() {
    assert_eq!(Mark::Checked.toggled(), Mark::Unchecked);
    assert_eq!(Mark::Unchecked.toggled(), Mark::Checked);
}
