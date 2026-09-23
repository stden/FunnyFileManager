//! Viewer-producing operations.
//!
use std::{
    fs, io,
    path::{Path, PathBuf},
};

use crate::fileload::{read_for_text_ui, Loaded};
use crate::models::{App, FileSize, Overlay, Viewer};
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

/// Show MCP server status and installation info (F11 / `:mcp`).
pub fn open_mcp_status(app: &mut App) -> io::Result<()> {
    let project = std::env::current_dir().unwrap_or_default();
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_default();
    open_mcp_status_in(app, &project, Path::new(&home));
    Ok(())
}

fn open_mcp_status_in(app: &mut App, project: &Path, home: &Path) {
    let mut lines = mcp_status_header(project);
    lines.extend(mcp_config_lines(project, home));

    lines.push(String::new());
    lines.push("Commands:".to_owned());
    lines.push("  ./install-mcp.sh    Install for all IDEs".to_owned());
    lines.push("  :mcp                Show this screen".to_owned());
    lines.push(String::new());
    lines.push("Tools:".to_owned());
    lines.extend(tool_list_lines());

    app.set_overlay(Overlay::Viewer(Viewer::report("MCP Server Status", lines)));
    app.set_status(Status::CommandOutput(
        "MCP Server — press Esc to close".to_owned(),
    ));
}

fn mcp_status_header(project: &Path) -> Vec<String> {
    let binary = project
        .join("mcp")
        .join("target")
        .join("release")
        .join("ffm-mcp");
    let binary_exists = binary.exists();

    vec![
        // Counted from the list itself, never written out by hand.
        format!("FFM MCP Server — {} AI Tools", TOOL_NAMES.len()),
        String::new(),
        format!(
            "Binary: {} {}",
            binary.display(),
            if binary_exists {
                "(found)"
            } else {
                "(not built)"
            }
        ),
        String::new(),
        "Supported IDEs:".to_owned(),
    ]
}

fn mcp_config_lines(project: &Path, home: &Path) -> Vec<String> {
    let configs = mcp_configs(project, home);
    let mut lines = Vec::new();
    for (name, path) in &configs {
        let has_ffm =
            path.exists() && fs::read_to_string(path).is_ok_and(|c| c.contains("ffm-mcp"));
        let status = if has_ffm {
            "✓ installed"
        } else if path.exists() {
            "⊘ not configured"
        } else {
            "— no config"
        };
        lines.push(format!("  {name:<16} {status}"));
    }

    lines
}

fn mcp_configs(project: &Path, home: &Path) -> Vec<(&'static str, PathBuf)> {
    vec![
        ("Claude Code", home.join(".claude.json")),
        ("Codex", home.join(".codex").join("mcp_config.json")),
        ("Gemini", project.join(".gemini").join("settings.json")),
        ("Junie", project.join(".junie").join("mcp.json")),
        (
            "Kiro",
            project.join(".kiro").join("settings").join("mcp.json"),
        ),
        ("VS Code/Cursor", project.join(".mcp.json")),
    ]
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
    // A NUL byte is the usual "this is binary" heuristic (git uses it too); the
    // line-oriented viewer can only render garbage for such a file.
    if bytes.contains(&0) {
        app.set_status(Status::BinaryFile(selected.display().to_string()));
        return;
    }
    app.set_status(Status::Viewing(selected.display().to_string()));
    // Two view histories survive the menu's arrival: the persisted one the
    // F11 overlay lists, and the session list the menu's own history reads.
    app.view_history.touch(&selected);
    app.viewed_files.retain(|path| path != &selected);
    app.viewed_files.push(selected.clone());
    if app.viewed_files.len() > 100 {
        app.viewed_files.remove(0);
    }
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
