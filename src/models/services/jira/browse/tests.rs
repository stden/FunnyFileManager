//! Purpose: Unit tests for `JiraDialog` mark toggling edge cases.

use std::path::PathBuf;

use serde_json::json;

use super::*;

#[test]
fn toggle_mark_handles_empty_list_and_unmarks_selected_issue() {
    let mut dialog = JiraDialog::new_setup(String::new(), "jql".into(), PathBuf::from("."));
    dialog.toggle_mark();
    assert!(dialog.marks.is_empty());

    dialog.issues.push(JiraIssue {
        key: "DEMO-1".into(),
        summary: "First".into(),
        issue_type: "Task".into(),
        status: "Open".into(),
        priority: "Medium".into(),
        updated: "2026-11-24".into(),
        assignee: "alice".into(),
        description: String::new(),
        comments: Vec::new(),
        raw: json!({"key": "DEMO-1"}),
    });
    dialog.toggle_mark();
    assert!(dialog.marks.contains("DEMO-1"));
    dialog.toggle_mark();
    assert!(dialog.marks.is_empty());
}
