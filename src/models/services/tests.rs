//! Purpose: Unit tests for `JiraDialog`, `McpRemoteDialog`, and `AiDialog` state transitions.

use std::path::PathBuf;

use serde_json::json;

use super::ai::AiDialog;
use super::jira::JiraDialog;
use super::mcp_remote::{McpFocus, McpRemoteDialog};
use crate::models::RowMotion;
use crate::values::ai::{AiError, AiProvider, AiResponse, AiSettings, WorkspaceContext};
use crate::values::jira::{ExportReport, JiraError, JiraIssue, JiraListing, JiraReply, JiraUser};
use crate::values::mcp_remote::{HttpMcpServer, McpCallResult, McpRemoteError, McpReply, McpTool};

fn sample_issue(key: &str, summary: &str) -> JiraIssue {
    JiraIssue {
        key: key.into(),
        summary: summary.into(),
        issue_type: "Story".into(),
        status: "Open".into(),
        priority: "High".into(),
        updated: "2026-11-22".into(),
        assignee: "alice".into(),
        description: "Body".into(),
        comments: Vec::new(),
        raw: json!({"key": key}),
    }
}

fn sample_listing() -> JiraListing {
    JiraListing {
        user: JiraUser {
            name: "alice".into(),
            ..JiraUser::default()
        },
        issues: vec![
            sample_issue("DEMO-1", "Alpha"),
            sample_issue("DEMO-2", "Beta"),
        ],
        total: 2,
        jql: "jql".into(),
        base_url: "https://jira.example.test".into(),
    }
}

#[test]
fn jira_dialog_filters_marks_and_navigates() {
    let setup = JiraDialog::new_setup("Need URL".into(), "jql".into(), PathBuf::from("."));
    assert!(!setup.is_busy() && setup.setup_error.is_some());
    let mut d = JiraDialog::new_ready(
        "jql".into(),
        20,
        PathBuf::from("."),
        "https://jira.example.test".into(),
    );
    let (req_id, _) = d.take_queued().expect("initial fetch");
    assert!(d.awaits(req_id));
    assert!(d
        .apply_reply(JiraReply::Fetched(Ok(sample_listing())))
        .expect("ok")
        .is_none());
    d.toggle_mark();
    d.move_cursor(RowMotion::Next);
    assert_eq!(d.selected_issue().map(|i| i.key.as_str()), Some("DEMO-2"));
    assert_eq!(d.selected_or_marked().len(), 1);
    d.type_filter('b');
    assert_eq!(d.filtered_indices(), vec![1]);
    d.erase_filter();
}

#[test]
fn jira_dialog_handles_export_and_error_replies() {
    let mut d = JiraDialog::new_setup(String::new(), "jql".into(), PathBuf::from("."));
    let exported = d.apply_reply(JiraReply::Exported(Ok(ExportReport {
        summary: "done".into(),
        output_dir: PathBuf::from("."),
        written: 2,
        skipped: 0,
    })));
    assert!(exported.expect("ok").is_some());
    assert!(d
        .apply_reply(JiraReply::Fetched(Err(JiraError::Disabled)))
        .is_err());
}

#[test]
fn mcp_remote_dialog_lists_tools_and_moves_cursors() {
    assert!(!McpRemoteDialog::new(Vec::new()).is_busy());
    let srv = HttpMcpServer {
        name: "svc".into(),
        url: "https://mcp.example.test".into(),
        ..HttpMcpServer::default()
    };
    let mut d = McpRemoteDialog::new(vec![srv]);
    let (id, _) = d.take_queued().expect("queued list");
    assert!(d.awaits(id));
    d.move_cursor(RowMotion::Next);
    let tools = vec![McpTool {
        name: "search".into(),
        description: "Find".into(),
        input_schema: json!({}),
    }];
    assert!(d
        .apply_reply(McpReply::Tools("svc".into(), Ok(tools)))
        .is_ok());
    assert_eq!(d.focus, McpFocus::Tools);
    d.move_cursor(RowMotion::Previous);
    assert_eq!(d.selected_tool().map(|t| t.name.as_str()), Some("search"));
}

#[test]
fn mcp_remote_dialog_applies_call_and_error_replies() {
    let mut d = McpRemoteDialog::new(Vec::new());
    let call = McpCallResult {
        server: "svc".into(),
        tool: "search".into(),
        is_error: true,
        text: "line1\nline2".into(),
    };
    assert!(d.apply_reply(McpReply::Called(Ok(call))).is_ok());
    d.move_cursor(RowMotion::Next);
    assert!(d
        .apply_reply(McpReply::Called(Err(McpRemoteError::Disabled)))
        .is_err());
}

#[test]
fn ai_dialog_cycles_providers_queues_prompts_and_handles_replies() {
    let mut d = AiDialog::new(AiSettings::default());
    assert_eq!(d.provider, AiProvider::Anthropic);
    d.cycle_provider();
    assert_eq!(d.provider, AiProvider::OpenAi);
    assert!(!d.queue_prompt(WorkspaceContext::default()));
    d.prompt.insert_char('h');
    assert!(d.queue_prompt(WorkspaceContext::default()) && d.is_busy());
    let (id, _) = d.take_queued().expect("queued");
    assert!(d.awaits(id));
    d.apply_reply(Ok(AiResponse {
        provider: AiProvider::OpenAi,
        model: "gpt-5".into(),
        text: "Run `ls`".into(),
        command: Some("ls".into()),
        input_tokens: Some(4),
        output_tokens: Some(2),
    }));
    assert!(!d.is_busy() && d.last_command.as_deref() == Some("ls"));
    d.apply_reply(Err(AiError::Disabled));
    d.clear();
    assert_eq!(d.messages.len(), 0);
}
