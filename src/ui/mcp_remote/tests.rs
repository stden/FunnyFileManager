//! Purpose: Rendering tests for the HTTP MCP services browser, call prompt, and output view.

use std::collections::BTreeMap;

use crate::lineedit::LineEditor;
use crate::models::services::mcp_remote::{McpFocus, McpMode, McpRemoteDialog};
use crate::models::{App, Overlay, Panel};
use crate::ui::overlays::render_overlay_text as render_text;
use crate::values::mcp_remote::{HttpMcpServer, McpTool};
use serde_json::json;

fn seeded_mcp_dialog() -> McpRemoteDialog {
    let mut dialog = McpRemoteDialog::new(vec![HttpMcpServer {
        name: "docs".into(),
        url: "https://mcp.example.test/docs".into(),
        headers: BTreeMap::new(),
        disabled_tools: Vec::new(),
        source: "ffm.yml".into(),
        tls_verify: true,
        ca_cert: None,
    }]);
    dialog.tools = vec![McpTool {
        name: "search".into(),
        description: "Find docs".into(),
        input_schema: json!({}),
    }];
    dialog.focus = McpFocus::Tools;
    dialog
}

#[test]
fn renders_mcp_browse_mode() {
    let temp = tempfile::TempDir::new().expect("temp");
    let panel = Panel::from_entries(temp.path().to_path_buf(), Vec::new());
    let mut app = App::with_panels(panel.clone(), panel);
    app.set_overlay(Overlay::McpServices(Box::new(seeded_mcp_dialog())));
    assert!(render_text(&mut app).contains("docs -> https://mcp.example.test/docs"));
}

#[test]
fn renders_mcp_call_input_and_output_modes() {
    let temp = tempfile::TempDir::new().expect("temp");
    let panel = Panel::from_entries(temp.path().to_path_buf(), Vec::new());
    let mut app = App::with_panels(panel.clone(), panel);
    let mut dialog = seeded_mcp_dialog();
    dialog.mode = McpMode::CallInput {
        tool: "search".into(),
        args: LineEditor::new("{\"q\":\"rust\"}"),
        notice: "Check syntax".into(),
    };
    app.set_overlay(Overlay::McpServices(Box::new(dialog.clone())));
    assert!(render_text(&mut app).contains("Check syntax"));

    dialog.mode = McpMode::Output {
        title: "docs:search".into(),
        lines: vec!["Result body".into()],
        scroll: 0,
    };
    app.set_overlay(Overlay::McpServices(Box::new(dialog)));
    assert!(render_text(&mut app).contains("Result body"));
}

#[test]
fn renders_empty_mcp_dialog_and_servers_focus() {
    let temp = tempfile::TempDir::new().expect("temp");
    let panel = Panel::from_entries(temp.path().to_path_buf(), Vec::new());
    let mut app = App::with_panels(panel.clone(), panel);
    app.set_overlay(Overlay::McpServices(Box::new(McpRemoteDialog::new(
        Vec::new(),
    ))));
    let empty_text = render_text(&mut app);
    assert!(empty_text.contains("Focus: Servers"));
    assert!(empty_text.contains("No HTTP MCP services discovered"));
    assert!(empty_text.contains("Press Enter on a service"));

    let mut dialog = seeded_mcp_dialog();
    dialog.servers.push(HttpMcpServer {
        name: "extra".into(),
        url: "https://mcp.example.test/extra".into(),
        ..HttpMcpServer::default()
    });
    dialog.focus = McpFocus::Servers;
    app.set_overlay(Overlay::McpServices(Box::new(dialog)));
    let multi = render_text(&mut app);
    assert!(multi.contains("> docs ->"));
    assert!(multi.contains("  extra ->"));
}

#[test]
fn renders_call_input_without_notice_and_empty_output() {
    let temp = tempfile::TempDir::new().expect("temp");
    let panel = Panel::from_entries(temp.path().to_path_buf(), Vec::new());
    let mut app = App::with_panels(panel.clone(), panel);
    let mut dialog = seeded_mcp_dialog();
    dialog.mode = McpMode::CallInput {
        tool: "search".into(),
        args: LineEditor::new("{}"),
        notice: String::new(),
    };
    app.set_overlay(Overlay::McpServices(Box::new(dialog.clone())));
    assert!(render_text(&mut app).contains("Enter Run tool | Esc Back"));

    dialog.mode = McpMode::Output {
        title: "docs:search".into(),
        lines: Vec::new(),
        scroll: 0,
    };
    app.set_overlay(Overlay::McpServices(Box::new(dialog)));
    assert!(render_text(&mut app).contains("(empty tool output)"));
}
