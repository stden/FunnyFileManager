//! Viewer-producing operations.
//!
use std::{io, path::PathBuf};

mod image;
mod mcp_status;

use crate::fileload::{read_for_text_ui, Loaded};
use crate::models::{App, FileSize, Overlay, Viewer};
use crate::status::Status;
use crate::utils::MAX_FILE_SIZE;
use mcp_status::mcp_client_lines;

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

pub fn handle_view(app: &mut App) -> io::Result<()> {
    if let Some(path) = image::selected(app) {
        image::open(app, path);
        return Ok(());
    }
    if crate::ops::vault::open_vault_if_kdbx(app) {
        return Ok(());
    }
    if app.get_active_panel().selected_is_parent_dir() {
        measure_panel_directories(app.get_active_panel());
        return Ok(());
    }
    let Some(selected) = app.get_active_panel().selected_path() else {
        return Ok(());
    };
    view_path(app, selected)
}

fn measure_panel_directories(panel: &mut crate::models::Panel) {
    let targets = panel.directory_targets();
    let sizes: Vec<_> = std::thread::scope(|scope| {
        let mut handles = Vec::with_capacity(targets.len());
        for (name, path) in targets {
            handles.push(scope.spawn(move || (name, crate::stats::analyse(&path).total)));
        }
        handles
            .into_iter()
            .filter_map(|handle| handle.join().ok())
            .collect()
    });
    panel.set_directory_sizes(&sizes);
}

pub fn view_path(app: &mut App, selected: PathBuf) -> io::Result<()> {
    // Far measures a directory in-place in the panel's Size column rather than
    // opening a viewer or modal dialog.
    if selected.is_dir() {
        let stats = crate::stats::analyse(&selected);
        app.get_active_panel()
            .set_directory_size_at(&selected, stats.total);
        return Ok(());
    }
    if crate::images::is_supported_path(&selected) {
        image::open(app, selected);
        return Ok(());
    }
    // `read_for_text_ui` owns both guards F3 needs: it refuses anything that is
    // not a regular file (a FIFO would block the whole event loop in
    // `File::open`, /dev/zero would be read until the OOM killer fires — both
    // report `len() == 0`, so the size check alone never saw them), and it
    // bounds the read itself.
    let loaded = read_for_text_ui(&selected);
    let bytes = match loaded.map_err(|error| crate::utils::FailedAt::wrap(&selected, error))? {
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
    let tab_size = app
        .settings
        .number(crate::menu::SettingsPage::Viewer)
        .max(1);
    for line in &mut viewer.lines {
        if memchr::memchr(b'\t', line.as_bytes()).is_some() {
            *line = expand_viewer_tabs(line, tab_size);
        }
    }
    app.set_overlay(Overlay::Viewer(viewer));
}

fn expand_viewer_tabs(line: &str, tab_size: usize) -> String {
    use unicode_width::UnicodeWidthChar;
    let mut out = String::with_capacity(line.len() + tab_size);
    let mut col = 0;
    for ch in line.chars() {
        if ch == '\t' {
            let next = (col / tab_size + 1) * tab_size;
            out.extend(std::iter::repeat_n(' ', next - col));
            col = next;
        } else {
            out.push(ch);
            col += UnicodeWidthChar::width(ch).unwrap_or(0);
        }
    }
    out
}

// FIFOs and device nodes are the inputs these tests need; neither exists on
// Windows, so the whole module is Unix-only.
#[cfg(all(test, unix))]
mod tests_special_files;

#[cfg(test)]
mod tests;
