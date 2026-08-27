//! Viewer-producing operations.
//!
use crate::scroll::ScrollTop;
use std::{fs, io, path::PathBuf};

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
/// separators still fit the viewer's inner width on an 80-column terminal.
const TOOLS_PER_LINE: usize = 3;

/// The tool list as indented, comma-separated viewer lines.
fn tool_list_lines() -> Vec<String> {
    TOOL_NAMES
        .chunks(TOOLS_PER_LINE)
        .map(|chunk| format!("  {}", chunk.join(", ")))
        .collect()
}

/// Show MCP server status and installation info (F9 / `:mcp`).
pub fn open_mcp_status(app: &mut App) -> io::Result<()> {
    let binary = std::env::current_dir()
        .unwrap_or_default()
        .join("mcp")
        .join("target")
        .join("release")
        .join("ffm-mcp");
    let binary_exists = binary.exists();

    let mut lines = vec![
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
    ];

    // Check each IDE config
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_default();
    let project = std::env::current_dir().unwrap_or_default();

    let configs: Vec<(&str, PathBuf)> = vec![
        ("Claude Code", PathBuf::from(&home).join(".claude.json")),
        (
            "Codex",
            PathBuf::from(&home).join(".codex").join("mcp_config.json"),
        ),
        ("Gemini", project.join(".gemini").join("settings.json")),
        ("Junie", project.join(".junie").join("mcp.json")),
        (
            "Kiro",
            project.join(".kiro").join("settings").join("mcp.json"),
        ),
        ("VS Code/Cursor", project.join(".mcp.json")),
    ];

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

    lines.push(String::new());
    lines.push("Commands:".to_owned());
    lines.push("  ./install-mcp.sh    Install for all IDEs".to_owned());
    lines.push("  :mcp                Show this screen".to_owned());
    lines.push(String::new());
    lines.push("Tools:".to_owned());
    lines.extend(tool_list_lines());

    app.set_overlay(Overlay::Viewer(Viewer {
        title: "MCP Server Status".to_owned(),
        lines,
        scroll: ScrollTop::TOP,
    }));
    app.set_status(Status::CommandOutput(
        "MCP Server — press Esc to close".to_owned(),
    ));
    Ok(())
}

pub fn handle_view(app: &mut App) -> io::Result<()> {
    let Some(selected) = app.get_active_panel().selected_path() else {
        return Ok(());
    };

    if selected.is_dir() {
        app.set_status(Status::CannotViewDir);
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
    // A NUL byte is the usual "this is binary" heuristic (git uses it too); the
    // line-oriented viewer can only render garbage for such a file.
    if bytes.contains(&0) {
        app.set_status(Status::BinaryFile(selected.display().to_string()));
        return Ok(());
    }
    // Lossy, so a non-UTF-8 *text* file (cp1251, latin-1) still opens instead of
    // failing the whole action with "stream did not contain valid UTF-8".
    let content = String::from_utf8_lossy(&bytes);
    let mut lines: Vec<String> = content
        .lines()
        .map(std::string::ToString::to_string)
        .collect();
    if lines.is_empty() {
        lines.push(String::from("<empty file>"));
    }

    app.set_overlay(Overlay::Viewer(Viewer {
        title: selected
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string(),
        lines,
        scroll: ScrollTop::TOP,
    }));
    app.set_status(Status::Viewing(selected.display().to_string()));
    Ok(())
}

// FIFOs and device nodes are the inputs these tests need; neither exists on
// Windows, so the whole module is Unix-only.
#[cfg(all(test, unix))]
mod tests_special_files;

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_ops_mcp_status() -> io::Result<()> {
        let mut app = App::new()?;
        open_mcp_status(&mut app)?;
        assert!(app.get_viewer().is_some());
        Ok(())
    }

    /// When the release binary sits at the path F9 probes, the status line says
    /// `(found)` instead of `(not built)`. Uses a process-wide lock + temporary
    /// cwd so parallel tests that also touch env/`current_dir` cannot race.
    #[test]
    fn open_mcp_status_reports_binary_found_when_present() -> io::Result<()> {
        let temp = TempDir::new()?;
        let binary = temp.path().join("mcp/target/release/ffm-mcp");
        fs::create_dir_all(binary.parent().expect("parent"))?;
        fs::write(&binary, b"fake-mcp-binary")?;

        let _lock = crate::test_support::env_lock();
        let previous = std::env::current_dir()?;
        std::env::set_current_dir(temp.path())?;

        let mut app = App::new()?;
        open_mcp_status(&mut app)?;
        let text = app.get_viewer().expect("viewer").lines.join("\n");

        std::env::set_current_dir(previous)?;

        assert!(
            text.contains("(found)"),
            "binary present must be reported as found: {text}"
        );
        Ok(())
    }

    /// Header and list are both derived from `TOOL_NAMES`; this pins that they
    /// stay derived. The hand-written copies drifted: a "38 AI Tools" header
    /// above a list of 41 names that still advertised `open_editor`, a tool the
    /// server had already dropped.
    #[test]
    fn test_mcp_status_header_count_matches_the_tools_it_lists() -> io::Result<()> {
        let mut app = App::new()?;
        open_mcp_status(&mut app)?;
        let viewer = app.get_viewer().expect("MCP status opens the viewer");

        let header = viewer.lines.first().expect("the header line");
        let claimed: usize = header
            .split_whitespace()
            .find_map(|word| word.parse().ok())
            .expect("the header must state how many tools there are");

        let tools_at = viewer
            .lines
            .iter()
            .position(|line| line == "Tools:")
            .expect("the tool list section");
        let listed: Vec<&str> = viewer.lines[tools_at + 1..]
            .iter()
            .flat_map(|line| line.split(','))
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .collect();

        assert_eq!(
            claimed,
            listed.len(),
            "header {header:?} disagrees with the list it heads: {listed:?}"
        );
        assert!(
            !listed.contains(&"open_editor"),
            "the screen must not advertise a tool the server no longer registers"
        );
        Ok(())
    }

    #[test]
    fn test_ops_view_file() -> io::Result<()> {
        let temp = TempDir::new()?;
        let file = temp.path().join("file.txt");
        fs::write(&file, b"content")?;

        let mut app = App::new()?;
        app.left_panel.path = temp.path().to_path_buf();
        app.left_panel.load_entries()?;
        let file_idx = app
            .left_panel
            .entries
            .iter()
            .position(|e| e.name == "file.txt")
            .unwrap();
        app.left_panel.select(Some(file_idx));

        handle_view(&mut app)?;
        assert!(app.get_viewer().is_some());

        // Nonexistent file selection
        app.left_panel.select(None);
        handle_view(&mut app)?;

        // Directory selection
        let subdir = temp.path().join("subdir");
        fs::create_dir(&subdir)?;
        app.left_panel.load_entries()?;
        let idx = app
            .left_panel
            .entries
            .iter()
            .position(|e| e.name == "subdir")
            .unwrap();
        app.left_panel.select(Some(idx));
        handle_view(&mut app)?;
        assert_eq!(app.status_text(), "Cannot view a directory");

        Ok(())
    }
}
