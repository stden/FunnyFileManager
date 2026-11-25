//! Purpose: Key routing tests for the HTTP MCP services browser and tool call editor.

use super::{app_in, press};
use crate::models::services::mcp_remote::{McpFocus, McpMode, McpRemoteDialog};
use crate::models::Overlay;
use crate::values::mcp_remote::{HttpMcpServer, McpRequest, McpTool};
use crossterm::event::{KeyCode, KeyModifiers};
use serde_json::json;
use std::collections::BTreeMap;

fn seed_mcp(dir: &std::path::Path) -> crate::models::App {
    let mut app = app_in(dir);
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
        input_schema: json!({"type": "object", "required": ["q"], "properties": {"q": {"type": "string"}}}),
    }];
    drop(dialog.take_queued());
    app.set_overlay(Overlay::McpServices(Box::new(dialog)));
    app
}

#[test]
fn mcp_browse_call_input_output_and_save() {
    let temp = tempfile::TempDir::new().expect("temp");
    let mut app = seed_mcp(temp.path());
    press(&mut app, KeyCode::Enter, KeyModifiers::NONE);
    press(&mut app, KeyCode::F(5), KeyModifiers::NONE);
    press(&mut app, KeyCode::Char('r'), KeyModifiers::CONTROL);
    press(&mut app, KeyCode::Tab, KeyModifiers::NONE);
    press(&mut app, KeyCode::Tab, KeyModifiers::NONE);
    press(&mut app, KeyCode::Right, KeyModifiers::NONE);
    press(&mut app, KeyCode::Char('x'), KeyModifiers::NONE);
    press(&mut app, KeyCode::Down, KeyModifiers::NONE);
    press(&mut app, KeyCode::Enter, KeyModifiers::NONE);
    press(&mut app, KeyCode::Esc, KeyModifiers::NONE);
    press(&mut app, KeyCode::Enter, KeyModifiers::NONE);
    press(&mut app, KeyCode::Home, KeyModifiers::NONE);
    press(&mut app, KeyCode::Enter, KeyModifiers::NONE);
    let Overlay::McpServices(d) = &mut app.overlay else {
        panic!("mcp")
    };
    assert!(matches!(d.queued, Some((_, McpRequest::CallTool { .. }))));
    assert_eq!(d.focus, McpFocus::Tools);
    verify_output_and_editor(&mut app, temp.path());
}

fn verify_output_and_editor(app: &mut crate::models::App, dir: &std::path::Path) {
    if let Overlay::McpServices(d) = &mut app.overlay {
        d.mode = McpMode::Output {
            title: "docs:search".into(),
            lines: vec!["line 1".into(), "line 2".into()],
            scroll: 0,
        };
    }
    press(app, KeyCode::Down, KeyModifiers::NONE);
    press(app, KeyCode::Char('x'), KeyModifiers::NONE);
    press(app, KeyCode::F(2), KeyModifiers::NONE);
    press(app, KeyCode::Char('s'), KeyModifiers::CONTROL);
    assert!(dir.join("mcp-output.txt").exists());
    press(app, KeyCode::Esc, KeyModifiers::NONE);
    press(app, KeyCode::Left, KeyModifiers::NONE);
    let _home = crate::test_support::EnvVarGuard::set(crate::test_support::CONFIG_HOME, dir);
    press(app, KeyCode::F(4), KeyModifiers::NONE);
    assert!(matches!(app.overlay, Overlay::Editor(_)));
}

#[test]
fn mcp_escape_from_browse_closes_overlay() {
    let temp = tempfile::TempDir::new().expect("temp");
    let mut app = seed_mcp(temp.path());
    press(&mut app, KeyCode::Esc, KeyModifiers::NONE);
    assert!(matches!(app.overlay, Overlay::None));
}

#[test]
fn mcp_guards_ignore_wrong_overlay_and_non_input_modes() {
    let temp = tempfile::TempDir::new().expect("temp");
    let mut app = app_in(temp.path());
    crate::popup::services::mcp_remote::process(&mut app, KeyCode::Esc, KeyModifiers::NONE);
    assert!(crate::popup::services::mcp_remote::pick_args(&mut app.overlay).is_none());

    let mut dialog = McpRemoteDialog::new(Vec::new());
    dialog.mode = McpMode::Browse;
    let mut browse_overlay = Overlay::McpServices(Box::new(dialog));
    assert!(crate::popup::services::mcp_remote::pick_args(&mut browse_overlay).is_none());
}
