//! Viewer-producing operations.
//!
use std::{io, path::PathBuf};

use crate::fileload::{read_for_text_ui, Loaded};
use crate::mcp_clients::{server_lines, Host, ProjectDir, Scope};
use crate::models::{App, FileSize, Overlay, Viewer};
use crate::ops::mcp_install::survey;
use crate::status::Status;
use crate::utils::MAX_FILE_SIZE;

// The tool list comes from the MCP server crate verbatim.
//
// WHY `include!` and not a copy: `mcp` is a *binary* crate, so it cannot be
// depended on, and the hand-kept copy that used to live here drifted — it
// advertised `open_editor` long after that tool was deleted and claimed 38
// tools when 41 were registered. Sharing the one file makes the screen and the
// server unable to disagree; `mcp`'s `tool_names_match_the_registered_router`
// pins that file to the router it actually builds. Do not re-inline this list.
include!("../../mcp/src/tool_names.rs");

/// Names per line in the tool list — three of the longest names plus their
/// separators still fit the viewer's width on an 80-column terminal.
const TOOLS_PER_LINE: usize = 3;

/// The tool list as indented, comma-separated viewer lines.
fn tool_list_lines() -> Vec<String> {
    TOOL_NAMES
        .chunks(TOOLS_PER_LINE)
        .map(|chunk| format!("  {}", chunk.join(", ")))
        .collect()
}

/// Show MCP server status and installation info (F11 → MCP status, or
/// `:mcp`).
///
/// The same survey the Install MCP server dialog makes, for both scopes, on
/// the machine `app.live.mcp_host` describes and the active panel's folder.
pub fn open_mcp_status(app: &mut App) -> io::Result<()> {
    let folder = crate::ops::mcp_install::active_folder(app);
    let mut lines = vec![
        // Counted from the list itself, never written out by hand.
        format!("FFM MCP Server — {} AI Tools", TOOL_NAMES.len()),
        String::new(),
    ];
    lines.extend(mcp_client_lines(&app.live.mcp_host, folder));
    lines.extend([
        String::new(),
        "Commands:".to_owned(),
        "  F9 Commands > Install MCP server   Register ffm-mcp with these clients".to_owned(),
        "  :mcp                               Show this screen".to_owned(),
        String::new(),
        "Tools:".to_owned(),
    ]);
    lines.extend(tool_list_lines());

    app.set_overlay(Overlay::Viewer(Viewer::report("MCP Server Status", lines)));
    app.set_status(Status::CommandOutput(
        "MCP Server — press Esc to close".to_owned(),
    ));
    Ok(())
}

/// The server, the folder, and one line per client with its state at both
/// scopes, in the words the Install MCP server dialog uses.
fn mcp_client_lines(host: &Host, folder: Option<ProjectDir>) -> Vec<String> {
    let everywhere = survey(host, Scope::AllProjects, folder.clone());
    let here = folder
        .is_some()
        .then(|| survey(host, Scope::CurrentFolder, folder));
    let mut lines = server_lines(&everywhere.server);
    lines.push(everywhere.folder.as_ref().map_or_else(
        || "Folder: the active panel is not a local folder".to_owned(),
        |folder| format!("Folder: {}", crate::path_display::label(folder.as_path())),
    ));
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
    lines
}

pub fn handle_view(app: &mut App) -> io::Result<()> {
    if crate::ops::vault::open_vault_if_kdbx(app) {
        return Ok(());
    }

    let Some(selected) = app.get_active_panel().selected_path() else {
        return Ok(());
    };
    view_path(app, selected)
}

pub fn view_path(app: &mut App, selected: PathBuf) -> io::Result<()> {
    // Far measures a directory rather than refusing to open it: F3 on a folder
    // is how you find out what is in it and how big it is, and "cannot view a
    // directory" answers a question nobody asked. The same panel Ctrl+L shows,
    // aimed at the row under the cursor instead of at the panel's own path.
    if selected.is_dir() {
        let stats = crate::stats::analyse(&selected);
        app.set_overlay(Overlay::Information(Box::new(stats)));
        return Ok(());
    }
    // `read_for_text_ui` owns both guards F3 needs: it refuses anything that is
    // not a regular file (a FIFO would block the whole event loop in
    // `File::open`, /dev/zero would be read until the OOM killer fires — both
    // report `len() == 0`, so the size check alone never saw them), and it
    // bounds the read itself.
    let bytes = match read_for_text_ui(&selected)? {
        Loaded::Bytes(bytes) => bytes,
        Loaded::NotRegular => {
            app.set_status(Status::NotRegularFile(selected.display().to_string()));
            return Ok(());
        }
        Loaded::TooLarge(size) => {
            app.set_status(Status::FileTooLarge {
                megabytes: FileSize::from_bytes(size).megabytes(),
                limit_megabytes: MAX_FILE_SIZE.megabytes(),
            });
            return Ok(());
        }
    };
    show_file_bytes(app, selected, &bytes);
    Ok(())
}

fn show_file_bytes(app: &mut App, selected: PathBuf, bytes: &[u8]) {
    // The line-oriented viewer can only render garbage for a binary file.
    if crate::encoding::is_binary(bytes) {
        app.set_status(Status::BinaryFile(selected.display().to_string()));
        return;
    }
    app.set_status(Status::Viewing(selected.display().to_string()));
    // The one history Alt+F11 lists; F4 and Alt+F4 record into it too.
    app.view_history.touch(&selected);
    let mut viewer = Viewer::file(selected, bytes);
    let spaces = " ".repeat(app.settings.number(crate::menu::SettingsPage::Viewer));
    for line in &mut viewer.lines {
        *line = line.replace('\t', &spaces);
    }
    app.set_overlay(Overlay::Viewer(viewer));
}

// FIFOs and device nodes are the inputs these tests need; neither exists on
// Windows, so the whole module is Unix-only.
#[cfg(all(test, unix))]
mod tests_special_files;

#[cfg(test)]
mod tests;
