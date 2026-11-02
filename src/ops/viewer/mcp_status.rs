//! Purpose: Formats the MCP client survey lines for the MCP Server Status viewer.

use crate::mcp_clients::{server_lines, Host, ProjectDir, Scope, Survey};
use crate::ops::mcp_install::{problem_section, survey};

/// The server, the folder, and one line per client with its state at both
/// scopes, in the words the Install MCP server dialog uses.
pub(super) fn mcp_client_lines(host: &Host, folder: Option<ProjectDir>) -> Vec<String> {
    let everywhere = survey(host, Scope::AllProjects, folder.clone());
    let here = folder
        .is_some()
        .then(|| survey(host, Scope::CurrentFolder, folder));
    let mut lines = server_and_folder(&everywhere);
    lines.push(String::new());
    lines.push(format!(
        "{:<32}{:<25}{}",
        "Supported IDEs:",
        Scope::AllProjects.label(),
        Scope::CurrentFolder.label()
    ));
    for (index, status) in everywhere.clients.iter().enumerate() {
        let in_folder = here
            .as_ref()
            .and_then(|here| here.clients.get(index))
            .map_or_else(|| "— no folder".to_owned(), |status| status.state.label());
        lines.push(format!(
            "  {:<15} {:<13} {:<24} {in_folder}",
            status.client.name(),
            status.presence.label(),
            status.state.label()
        ));
    }
    let surveyed = here.iter().flat_map(|here| &here.clients);
    lines.extend(problem_section(everywhere.clients.iter().chain(surveyed)));
    lines
}

/// The server and the folder `survey` found, one line each.
fn server_and_folder(survey: &Survey) -> Vec<String> {
    let folder = survey.folder.as_ref().map_or_else(
        || "Folder: the active panel is not a local folder".to_owned(),
        |folder| format!("Folder: {}", crate::path_display::label(folder.as_path())),
    );
    server_lines(&survey.server)
        .iter()
        .map(ToString::to_string)
        .chain([folder])
        .collect()
}
