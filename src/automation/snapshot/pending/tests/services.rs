//! Purpose: Pending state snapshot tests for Jira, HTTP MCP, and AI overlays.

use super::*;
use crate::models::services::ai::AiDialog;
use crate::models::services::jira::JiraDialog;
use crate::models::services::mcp_remote::McpRemoteDialog;
use crate::values::ai::AiSettings;
use crate::values::jira::JiraRequest;
use crate::values::mcp_remote::HttpMcpServer;
use std::collections::BTreeMap;

#[test]
fn service_dialogs_are_pending_only_while_queued_or_in_flight() {
    let root = TempDir::new().unwrap();
    let mut app = application(&root);

    let mut jira = JiraDialog::new_setup(String::new(), "x".into(), root.path().to_path_buf());
    app.overlay = Overlay::Jira(Box::new(jira.clone()));
    assert!(!is_pending(&app));
    jira.queue(JiraRequest::Fetch {
        jql: "x".into(),
        page_size: 10,
    });
    app.overlay = Overlay::Jira(Box::new(jira));
    assert!(is_pending(&app));

    let mcp = McpRemoteDialog::new(vec![HttpMcpServer {
        name: "docs".into(),
        url: "https://mcp.example.test".into(),
        headers: BTreeMap::new(),
        disabled_tools: Vec::new(),
        source: "ffm.yml".into(),
        tls_verify: true,
        ca_cert: None,
    }]);
    app.overlay = Overlay::McpServices(Box::new(mcp));
    assert!(is_pending(&app));

    let ai = AiDialog::new(AiSettings::default());
    app.overlay = Overlay::Ai(Box::new(ai));
    assert!(!is_pending(&app));
}
