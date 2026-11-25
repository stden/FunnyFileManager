//! Purpose: Unit tests for Streamable HTTP MCP operations and discovery rules.

use serde_json::json;

use super::discover::{discover_for, live_overrides, unique_paths};
use super::*;
use crate::mcp_clients::Host;
use crate::models::services::mcp_remote::McpFocus;
use crate::models::Panel;
use crate::values::mcp_remote::{McpAccess, McpCallResult, McpRemoteError, McpTool};

#[test]
fn discovery_finds_active_folder_configs_and_respects_offline_access() {
    let temp = tempfile::TempDir::new().expect("temp");
    let dot_kiro = temp.path().join(".kiro/settings");
    std::fs::create_dir_all(&dot_kiro).expect("mkdir");
    std::fs::write(
        dot_kiro.join("mcp.json"),
        r#"{"mcpServers":{"svc":{"type":"http","url":"https://mcp.example.test/s"}}}"#,
    )
    .expect("write");
    let found = discover_for(
        &Host::default(),
        McpAccess::Offline,
        temp.path(),
        None,
        None,
        None,
    );
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].name, "svc");
    assert_eq!(live_overrides(McpAccess::Offline), (None, None));
    assert_eq!(
        unique_paths(vec![temp.path().to_path_buf(), temp.path().to_path_buf()]).len(),
        1
    );
}

#[test]
fn open_publish_call_and_save_output_work_through_app() {
    let temp = tempfile::TempDir::new().expect("temp");
    std::fs::write(
        temp.path().join(".mcp.json"),
        r#"{"mcpServers":{"svc":{"type":"http","url":"https://mcp.example.test/s"}}}"#,
    )
    .expect("mcp.json");
    let panel = Panel::new(temp.path().to_path_buf()).expect("panel");
    let mut app = App::with_panels(panel.clone(), panel);
    command(&mut app, McpServicesCommand::Browse).expect("open");
    let (generation, id) = match &mut app.overlay {
        Overlay::McpServices(d) => (
            Arc::clone(&d.generation),
            d.take_queued().expect("queued").0,
        ),
        _ => panic!("expected McpServices"),
    };
    let tool = McpTool {
        name: "search".into(),
        description: "Search".into(),
        input_schema: json!({"type": "object", "required": ["q"], "properties": {"q": {"type": "string"}}}),
    };
    publish(
        &mut app,
        &generation,
        id,
        McpReply::Tools("svc".into(), Ok(vec![tool])),
    );
    open_call_input(&mut app);
    submit_call(&mut app);
    verify_call_output_and_reload(&mut app, temp.path());
}

fn verify_call_output_and_reload(app: &mut App, dir: &std::path::Path) {
    let (g2, id2) = match &mut app.overlay {
        Overlay::McpServices(d) => (Arc::clone(&d.generation), d.take_queued().expect("call").0),
        _ => panic!("expected call"),
    };
    let out = McpCallResult {
        server: "svc".into(),
        tool: "search".into(),
        is_error: false,
        text: "result body".into(),
    };
    publish(app, &g2, id2, McpReply::Called(Ok(out)));
    save_output(app).expect("save output");
    assert!(dir.join("mcp-output.txt").is_file());
    if let Overlay::McpServices(d) = &mut app.overlay {
        d.focus = McpFocus::Servers;
    }
    submit_call(app);
    queue_list_tools(app);
    publish(
        app,
        &g2,
        id2,
        McpReply::Called(Err(McpRemoteError::Disabled)),
    );
    let _home = crate::test_support::EnvVarGuard::set(crate::test_support::CONFIG_HOME, dir);
    command(app, McpServicesCommand::Edit).expect("edit");
    assert!(matches!(app.overlay, Overlay::Editor(_)));
}

fn empty_app(dir: &std::path::Path) -> App {
    let panel = Panel::new(dir.to_path_buf()).expect("panel");
    App::with_panels(panel.clone(), panel)
}

#[test]
fn ops_guards_ignore_wrong_overlay_and_empty_selections() {
    let temp = tempfile::TempDir::new().expect("temp");
    let mut app = empty_app(temp.path());
    queue_list_tools(&mut app);
    open_call_input(&mut app);
    submit_call(&mut app);
    save_output(&mut app).expect("save without overlay");
    publish(
        &mut app,
        &Arc::new(()),
        McpRequestId::default(),
        McpReply::Called(Err(McpRemoteError::Worker)),
    );
}

#[test]
fn ops_guards_ignore_empty_dialog_selections() {
    let temp = tempfile::TempDir::new().expect("temp");
    let mut app = empty_app(temp.path());
    app.set_overlay(Overlay::McpServices(Box::new(McpRemoteDialog::new(
        Vec::new(),
    ))));
    queue_list_tools(&mut app);
    open_call_input(&mut app);
    submit_call(&mut app);
    save_output(&mut app).expect("save outside output mode");
    if let Overlay::McpServices(d) = &mut app.overlay {
        d.mode = McpMode::CallInput {
            tool: "t".into(),
            args: LineEditor::new("{}"),
            notice: String::new(),
        };
    }
    submit_call(&mut app);
}

fn call_input_app(dir: &std::path::Path, raw_args: &str) -> App {
    let mut app = empty_app(dir);
    let srv = crate::values::mcp_remote::HttpMcpServer {
        name: "svc".into(),
        url: "https://mcp.example.test/s".into(),
        ..Default::default()
    };
    let mut dialog = McpRemoteDialog::new(vec![srv]);
    dialog.mode = McpMode::CallInput {
        tool: "search".into(),
        args: LineEditor::new(raw_args),
        notice: String::new(),
    };
    app.set_overlay(Overlay::McpServices(Box::new(dialog)));
    app
}

#[test]
fn submit_call_handles_invalid_and_blank_json_arguments() {
    let temp = tempfile::TempDir::new().expect("temp");
    let mut app = call_input_app(temp.path(), "{bad");
    submit_call(&mut app);
    let Overlay::McpServices(d) = &mut app.overlay else {
        panic!("expected McpServices")
    };
    assert!(
        matches!(&d.mode, McpMode::CallInput { notice, .. } if notice.contains("Invalid JSON"))
    );

    let mut blank_app = call_input_app(temp.path(), "   ");
    submit_call(&mut blank_app);
    let Overlay::McpServices(d2) = &mut blank_app.overlay else {
        panic!("expected McpServices")
    };
    assert!(
        matches!(d2.take_queued(), Some((_, McpRequest::CallTool { arguments, .. })) if arguments == json!({}))
    );
}
