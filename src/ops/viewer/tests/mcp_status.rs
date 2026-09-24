//! Purpose: The `:mcp` screen: the server, the clients and the tool count.

use super::*;

#[test]
fn test_ops_mcp_status() -> io::Result<()> {
    let mut app = App::new()?;
    open_mcp_status(&mut app)?;
    assert!(app.get_viewer().is_some());
    Ok(())
}

/// An `App` on a fake machine, its panels on the machine's project folder.
fn mcp_app(machine: &crate::test_support::FakeMachine) -> App {
    let mut app = App::with_panels(
        crate::models::Panel::new(machine.folder()).expect("panel"),
        crate::models::Panel::new(machine.folder()).expect("panel"),
    );
    app.live.mcp_host = machine.host();
    app
}

fn mcp_screen(app: &mut App) -> String {
    open_mcp_status(app).expect("the status screen opens");
    app.get_viewer().expect("viewer").lines.join("\n")
}

/// The server is looked for where `cargo install --path mcp` puts it — next
/// to `ffm` — and its path is shown.
#[test]
fn open_mcp_status_reports_the_server_next_to_ffm() {
    let machine = crate::test_support::FakeMachine::new().expect("fake machine");
    let server = machine.install_server().expect("server");
    let text = mcp_screen(&mut mcp_app(&machine));
    assert!(
        text.contains(&format!("Server: {}", server.display())),
        "the server beside ffm must be reported: {text}"
    );
    assert!(text.contains(&format!("Folder: {}", machine.folder().display())));
}

/// A missing server says so, and how to install it.
#[test]
fn open_mcp_status_explains_a_missing_server() {
    let machine = crate::test_support::FakeMachine::new().expect("fake machine");
    let text = mcp_screen(&mut mcp_app(&machine));
    assert!(
        text.contains("ffm-mcp was not found next to ffm or on PATH."),
        "{text}"
    );
    assert!(text.contains("cargo install --path mcp"), "{text}");
    assert!(!text.contains("install-mcp.sh"), "{text}");
}

/// A config that exists but was never touched by the installer must be
/// reported as "not configured", distinct from both "registered" and "no
/// config" — per client, and per scope.
#[test]
fn open_mcp_status_reports_not_configured_when_a_config_exists_without_ffm_mcp() {
    let machine = crate::test_support::FakeMachine::new().expect("fake machine");
    machine.install("claude").expect("claude");
    fs::write(machine.folder().join(".mcp.json"), b"{}").expect("config");
    let text = mcp_screen(&mut mcp_app(&machine));
    let claude = text
        .lines()
        .find(|line| line.trim_start().starts_with("Claude Code"))
        .expect("a Claude Code row");
    assert!(
        claude.ends_with("⊘ not configured"),
        "the folder column of Claude Code must read not configured: {claude}"
    );
    assert!(claude.contains("— no config"), "{claude}");
}

/// Every client of the shared table is listed with both scopes, and the
/// clients FFM cannot register with are gone.
#[test]
fn open_mcp_status_lists_the_four_clients_at_both_scopes() {
    let machine = crate::test_support::FakeMachine::new().expect("fake machine");
    let text = mcp_screen(&mut mcp_app(&machine));
    assert!(text.contains("Supported IDEs:"), "{text}");
    assert!(text.contains("All projects"), "{text}");
    assert!(text.contains("Current folder"), "{text}");
    for name in ["Claude Code", "Codex", "Junie", "Kiro"] {
        assert!(text.contains(name), "{name}: {text}");
    }
    for gone in ["Gemini", "VS Code", "mcp/target/release"] {
        assert!(!text.contains(gone), "{gone}: {text}");
    }
    assert!(text.contains("F9 Commands > Install MCP server"), "{text}");
}

/// A panel that is not on a local folder has no project column.
#[test]
fn open_mcp_status_without_a_local_folder() {
    let machine = crate::test_support::FakeMachine::new().expect("fake machine");
    let mut app = mcp_app(&machine);
    app.get_active_panel().path = machine.folder().join("gone");
    let text = mcp_screen(&mut app);
    assert!(
        text.contains("Folder: the active panel is not a local folder"),
        "{text}"
    );
    assert!(text.contains("— no folder"), "{text}");
}

/// A `NetBox` panel's path still names the local folder it hides; the screen
/// must not report that folder, which is not on screen.
#[test]
fn open_mcp_status_over_a_netbox_panel_has_no_folder() {
    let machine = crate::test_support::FakeMachine::new().expect("fake machine");
    let mut app = mcp_app(&machine);
    let side = app.active_panel;
    crate::test_support::netbox::open_empty_sessions(&mut app, side);
    let text = mcp_screen(&mut app);
    assert!(
        text.contains("Folder: the active panel is not a local folder"),
        "{text}"
    );
    assert!(text.contains("— no folder"), "{text}");
    assert!(
        !text.contains(&format!("Folder: {}", machine.folder().display())),
        "{text}"
    );
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
