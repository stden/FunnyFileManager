//! Purpose: The MCP status viewer: contents, missing binary and unconfigured IDEs.

use super::*;

// ─── open_mcp_status (handlers.rs lines 606-689) ──────────────────

#[test]
fn test_open_mcp_status_sets_viewer() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = make_app(temp.path())?;

    open_mcp_status(&mut app)?;

    // Verify viewer was set
    assert!(app.get_viewer().is_some());
    let viewer = app.get_viewer().unwrap();
    assert_eq!(viewer.title(), "MCP Server Status");
    assert_eq!(viewer.scroll, ScrollTop::TOP);
    assert!(!viewer.lines.is_empty());

    // Check key content
    let all_text = viewer.lines.join("\n");
    assert!(all_text.contains("FFM MCP Server"));
    assert!(all_text.contains("Server:"));
    assert!(all_text.contains("Supported IDEs:"));
    for client in ["Claude Code", "Codex", "Junie", "Kiro"] {
        assert!(all_text.contains(client), "{client} is listed");
    }
    // FFM cannot register with these, so the screen no longer lists them.
    assert!(!all_text.contains("Gemini"));
    assert!(!all_text.contains("VS Code/Cursor"));
    assert!(all_text.contains("Commands:"));
    assert!(all_text.contains("Tools:"));
    assert!(all_text.contains("list_directory"));
    assert!(all_text.contains("s3_copy"));

    // Check status message
    assert!(app.status_text().contains("MCP Server"));

    Ok(())
}

#[test]
fn test_open_mcp_status_binary_not_found() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = make_app(temp.path())?;

    // Binary won't exist at the temp dir
    open_mcp_status(&mut app)?;

    let all_text = app.get_viewer().unwrap().lines.join("\n");
    // A new `App` looks at no machine, so no server is found anywhere.
    assert!(all_text.contains("ffm-mcp was not found next to ffm or on PATH."));

    Ok(())
}

#[test]
fn test_event_loop_f9_mcp_status() -> io::Result<()> {
    let temp = TempDir::new()?;
    fs::write(temp.path().join("file.txt"), b"data")?;
    let mut app = make_app(temp.path())?;
    app.left_panel.select(Some(0));

    run_events(
        app,
        vec![
            Event::Key(key_event(KeyCode::F(9))),
            Event::Key(key_event(KeyCode::Esc)), // close viewer
            Event::Key(key_event(KeyCode::F(10))),
            // F10 now asks before quitting; Enter answers Yes.
            Event::Key(key_event(KeyCode::Enter)),
        ],
    )
}

// ─── MCP status IDE config branches (handlers.rs lines 659-662) ─────

#[test]
fn test_open_mcp_status_ide_config_not_configured() -> io::Result<()> {
    let temp = TempDir::new()?;

    // Claude Code's project config, without "ffm-mcp": "not configured".
    // The screen reads the active panel's folder, so no working directory
    // has to change for it to be found.
    let mcp_json = temp.path().join(".mcp.json");
    let mut f = File::create(&mcp_json)?;
    write!(f, r#"{{"some_other_tool": true}}"#)?;

    let mut app = make_app(temp.path())?;
    open_mcp_status(&mut app)?;

    let all_text = app.get_viewer().unwrap().lines.join("\n");
    let claude = all_text
        .lines()
        .find(|line| line.trim_start().starts_with("Claude Code"))
        .expect("a Claude Code row");
    assert!(claude.ends_with("⊘ not configured"), "{claude}");

    Ok(())
}
