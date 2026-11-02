//! Purpose: The client table: unique rows, the paths each client documents,
//! the scopes and the notes the dialog prints under the rows.

use std::collections::BTreeSet;

use super::*;

/// The widest line a dialog on an 80-column screen can draw inside its chrome.
const DIALOG_WIDTH: usize = 72;

#[test]
fn the_table_lists_four_clients_with_unique_ids_names_and_programs() {
    assert_eq!(CLIENTS.len(), 4);
    let ids: BTreeSet<ClientId> = CLIENTS.iter().map(|client| client.id).collect();
    let names: BTreeSet<&str> = CLIENTS.iter().map(|client| client.name).collect();
    let programs: Vec<&str> = CLIENTS
        .iter()
        .flat_map(|client| client.programs.iter().copied())
        .collect();
    let unique_programs: BTreeSet<&str> = programs.iter().copied().collect();
    assert_eq!(ids.len(), 4);
    assert_eq!(names.len(), 4);
    assert_eq!(unique_programs.len(), programs.len());
}

#[test]
fn every_client_id_leads_back_to_its_own_row() {
    for client in &CLIENTS {
        assert_eq!(client.id.client(), client);
        assert_eq!(client.id.name(), client.name);
    }
}

#[test]
fn table_paths_are_relative_and_slash_separated() {
    for client in &CLIENTS {
        for path in [client.user_config, client.project_config, client.home_dir] {
            assert!(!path.starts_with('/'), "{path} is absolute");
            assert!(!path.contains('\\'), "{path} uses a backslash");
        }
    }
}

#[test]
fn the_paths_are_the_ones_each_client_documents() {
    let codex = ClientId::Codex.client();
    assert_eq!(codex.user_config, "config.toml");
    assert_eq!(codex.format, ConfigFormat::Toml);
    assert_eq!(
        ClientId::Junie.client().project_config,
        ".junie/mcp/mcp.json"
    );
    assert_eq!(
        ClientId::ClaudeCode.client().mechanism(Scope::AllProjects),
        Mechanism::ClaudeCli
    );
    assert_eq!(
        ClientId::ClaudeCode
            .client()
            .mechanism(Scope::CurrentFolder),
        Mechanism::EditFile
    );
    assert_eq!(
        ClientId::Kiro.client().config(Scope::CurrentFolder),
        ".kiro/settings/mcp.json"
    );
}

#[test]
fn every_note_fits_an_80_column_dialog() {
    for client in &CLIENTS {
        for scope in Scope::ALL {
            if let Some(note) = client.note(scope) {
                let line = format!("{}: {note}", client.name);
                assert!(line.chars().count() <= DIALOG_WIDTH, "{line}");
            }
        }
    }
}

#[test]
fn notes_warn_junie_and_kiro_at_all_projects_and_claude_and_codex_in_a_folder() {
    let noted = |scope| -> Vec<ClientId> {
        CLIENTS
            .iter()
            .filter(|client| client.note(scope).is_some())
            .map(|client| client.id)
            .collect()
    };
    assert_eq!(noted(Scope::AllProjects), [ClientId::Junie, ClientId::Kiro]);
    assert_eq!(
        noted(Scope::CurrentFolder),
        [ClientId::ClaudeCode, ClientId::Codex]
    );
}

#[test]
fn scope_labels_and_the_other_scope() {
    assert_eq!(Scope::default(), Scope::AllProjects);
    assert_eq!(Scope::AllProjects.label(), "All projects");
    assert_eq!(Scope::CurrentFolder.label(), "Current folder");
    assert_eq!(Scope::AllProjects.other(), Scope::CurrentFolder);
    assert_eq!(Scope::CurrentFolder.other(), Scope::AllProjects);
}

#[test]
fn a_note_line_names_its_client() {
    assert_eq!(
        ClientId::Kiro
            .client()
            .note_line(Scope::AllProjects)
            .as_deref(),
        Some("Kiro: may start ffm-mcp outside the project; prefer Current folder.")
    );
    assert_eq!(
        ClientId::Kiro.client().note_line(Scope::CurrentFolder),
        None
    );
    assert_eq!(notes(Scope::CurrentFolder).len(), 2);
}
