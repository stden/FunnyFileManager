//! Purpose: Edge-case unit tests for Jira ticket export and markdown rendering.

use serde_json::json;

use super::super::export::{export_tickets, write_file};
use super::super::markdown::render_issue_md;
use crate::values::jira::{JiraComment, JiraIssue};

fn blank_author_issue() -> JiraIssue {
    JiraIssue {
        key: "DEMO-55".into(),
        summary: "Edge cases".into(),
        issue_type: "Task".into(),
        status: "Open".into(),
        priority: "Low".into(),
        updated: "2026-11-22".into(),
        assignee: String::new(),
        description: String::new(),
        comments: vec![JiraComment {
            id: "1".into(),
            author_name: String::new(),
            author_key: String::new(),
            updated: "2026-11-22T01:02:03".into(),
            body: "   ".into(),
        }],
        raw: json!({"key": "DEMO-55"}),
    }
}

#[test]
fn render_and_export_handle_missing_author_blank_comment_and_io_error() {
    let issue = blank_author_issue();
    let rendered = render_issue_md("https://jira.example.test", &issue);
    assert!(rendered.contains("#### ? 2026-11-22"));

    let temp = tempfile::TempDir::new().expect("temp");
    let report = export_tickets(
        temp.path(),
        "https://jira.example.test",
        None,
        &[issue],
        false,
    )
    .expect("export without user");
    assert_eq!(report.written, 2);

    let err = write_file(temp.path(), "cannot overwrite directory").unwrap_err();
    assert!(matches!(err, crate::values::jira::JiraError::Io(_)));
}
