//! Unit tests for [`super`], moved out of the module so their
//! assertion-failure arms are not measured as unreached production lines.

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
/// `(found)` instead of `(not built)`. Paths are injected so parallel tests
/// never observe a temporary process-wide working directory.
#[test]
fn open_mcp_status_reports_binary_found_when_present() -> io::Result<()> {
    let temp = TempDir::new()?;
    let binary = temp.path().join("mcp/target/release/ffm-mcp");
    fs::create_dir_all(binary.parent().expect("parent"))?;
    fs::write(&binary, b"fake-mcp-binary")?;

    let mut app = App::new()?;
    open_mcp_status_in(&mut app, temp.path(), temp.path());
    let text = app.get_viewer().expect("viewer").lines.join("\n");

    assert!(
        text.contains("(found)"),
        "binary present must be reported as found: {text}"
    );
    Ok(())
}

/// A config file that exists but was never touched by the installer must be
/// reported as "not configured", distinct from both "installed" and "no
/// config" — the three states the F11 screen promises per IDE.
#[test]
fn open_mcp_status_reports_not_configured_when_a_config_exists_without_ffm_mcp() -> io::Result<()> {
    let temp = TempDir::new()?;
    // The VS Code / Cursor slot reads `.mcp.json` from the project directory;
    // give it a body that exists but names no MCP server at all.
    fs::write(temp.path().join(".mcp.json"), b"{}")?;

    let mut app = App::new()?;
    open_mcp_status_in(&mut app, temp.path(), temp.path());
    let text = app.get_viewer().expect("viewer").lines.join("\n");

    assert!(
        text.lines()
            .any(|line| line.contains("VS Code/Cursor") && line.contains("not configured")),
        "an existing config without ffm-mcp must read as not configured: {text}"
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
    assert!(app.left_panel.select_named("file.txt"));

    handle_view(&mut app)?;
    assert!(app.get_viewer().is_some());

    // Nonexistent file selection
    app.left_panel.select(None);
    handle_view(&mut app)?;

    // Directory selection
    let subdir = temp.path().join("subdir");
    fs::create_dir(&subdir)?;
    app.left_panel.load_entries()?;
    assert!(app.left_panel.select_named("subdir"));
    handle_view(&mut app)?;
    assert!(
        matches!(app.overlay, Overlay::Information(_)),
        "F3 on a directory measures it, as Far does"
    );

    Ok(())
}

#[test]
fn handle_view_records_the_file_behind_the_viewer() -> io::Result<()> {
    let temp = TempDir::new()?;
    let file = temp.path().join("latin1.txt");
    fs::write(&file, b"caf\xe9\n")?;

    let mut app = App::new()?;
    app.left_panel.path = temp.path().to_path_buf();
    app.left_panel.load_entries()?;
    let index = app
        .left_panel
        .entries
        .iter()
        .position(|e| e.name == "latin1.txt")
        .expect("listed");
    app.left_panel.select(Some(index));

    handle_view(&mut app)?;
    let viewer = app.get_viewer().expect("viewer open");
    assert_eq!(viewer.title(), "latin1.txt");
    assert_eq!(viewer.lines, vec!["caf\u{439}"]);
    let opened = viewer.viewed_file().expect("a file viewer knows its file");
    assert_eq!(opened.path, file);
    assert_eq!(opened.size.bytes(), 5);
    assert_eq!(opened.encoding, crate::encoding::TextEncoding::Windows1251);
    assert_eq!(app.status_text(), format!("Viewing {}", file.display()));
    Ok(())
}

/// UTF-16 text has a NUL in the high byte of every ASCII character. The
/// byte-order mark says so, and F3 must read the file rather than call it
/// binary — F4 already opens the same file.
#[test]
fn f3_opens_a_utf16_file_with_a_bom_instead_of_refusing_it_as_binary() -> io::Result<()> {
    let temp = TempDir::new()?;
    let file = temp.path().join("utf16le.txt");
    fs::write(&file, b"\xff\xfeH\x00e\x00l\x00l\x00o\x00\n\x00")?;
    let panel = crate::models::Panel::new(temp.path().to_path_buf())?;
    let mut app = App::with_panels(panel.clone(), panel);

    view_path(&mut app, file.clone())?;

    let viewer = app.get_viewer().expect("F3 opens the viewer");
    assert_eq!(viewer.lines, vec!["Hello"]);
    assert_eq!(app.status, Status::Viewing(file.display().to_string()));
    Ok(())
}

/// The mark only excuses the NULs of text it names: a NUL-bearing file with
/// no byte-order mark is still refused.
#[test]
fn f3_still_refuses_a_nul_bearing_file_without_a_bom() -> io::Result<()> {
    let temp = TempDir::new()?;
    let file = temp.path().join("blob.bin");
    fs::write(&file, b"MZ\x00\x01\x02")?;
    let panel = crate::models::Panel::new(temp.path().to_path_buf())?;
    let mut app = App::with_panels(panel.clone(), panel);

    view_path(&mut app, file.clone())?;

    assert!(app.get_viewer().is_none());
    assert_eq!(app.status, Status::BinaryFile(file.display().to_string()));
    Ok(())
}

#[test]
fn viewing_a_file_records_it_in_the_view_history() -> io::Result<()> {
    let temp = TempDir::new()?;
    let file = temp.path().join("seen.txt");
    fs::write(&file, b"content")?;
    let mut app = App::new()?;
    app.left_panel.path = temp.path().to_path_buf();
    app.left_panel.load_entries()?;
    assert!(app.left_panel.select_named("seen.txt"));
    handle_view(&mut app)?;
    assert_eq!(app.view_history.last(), Some(file.as_path()));
    Ok(())
}
