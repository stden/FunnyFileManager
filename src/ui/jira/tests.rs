//! Purpose: Rendering tests for Jira browse, detail reader, and JQL editor views.

use crate::lineedit::LineEditor;
use crate::models::services::jira::{JiraDialog, JiraMode};
use crate::models::{App, Overlay, Panel};
use crate::ui::overlays::render_overlay_text as render_text;
use crate::values::jira::JiraIssue;
use serde_json::json;

fn jira_app(dir: &std::path::Path, dialog: JiraDialog) -> App {
    let panel = Panel::from_entries(dir.to_path_buf(), Vec::new());
    let mut app = App::with_panels(panel.clone(), panel);
    app.set_overlay(Overlay::Jira(Box::new(dialog)));
    app
}

#[test]
fn renders_jira_setup_and_browse() {
    let temp = tempfile::TempDir::new().expect("temp");
    let mut dialog = JiraDialog::new_setup(
        "Need token".into(),
        "project = DEMO".into(),
        temp.path().to_path_buf(),
    );
    let mut app = jira_app(temp.path(), dialog.clone());
    assert!(render_text(&mut app).contains("Need token"));

    dialog.setup_error = None;
    dialog.issues = vec![JiraIssue {
        key: "DEMO-1".into(),
        summary: "Render story".into(),
        issue_type: "Story".into(),
        status: "Open".into(),
        priority: "High".into(),
        updated: "2026-11-22".into(),
        assignee: "alice".into(),
        description: "Body".into(),
        comments: Vec::new(),
        raw: json!({"key": "DEMO-1"}),
    }];
    dialog.marks.insert("DEMO-1".into());
    app.set_overlay(Overlay::Jira(Box::new(dialog)));
    assert!(render_text(&mut app).contains("DEMO-1"));
}

#[test]
fn renders_jira_detail_and_jql() {
    let temp = tempfile::TempDir::new().expect("temp");
    let mut dialog = JiraDialog::new_setup(
        String::new(),
        "project = DEMO".into(),
        temp.path().to_path_buf(),
    );
    dialog.mode = JiraMode::Detail {
        title: "DEMO-1".into(),
        lines: vec!["# Title".into()],
        scroll: 0,
    };
    let mut app = jira_app(temp.path(), dialog.clone());
    assert!(render_text(&mut app).contains("# Title"));

    dialog.mode = JiraMode::EditJql(LineEditor::new("assignee = currentUser()"));
    app.set_overlay(Overlay::Jira(Box::new(dialog)));
    assert!(render_text(&mut app).contains("Edit Jira JQL"));
}

fn sample_unmarked_issue() -> JiraIssue {
    JiraIssue {
        key: "DEMO-2".into(),
        summary: "Unmarked story".into(),
        issue_type: "Story".into(),
        status: "Open".into(),
        priority: "Medium".into(),
        updated: "2026-11-22".into(),
        assignee: "bob".into(),
        description: "Body".into(),
        comments: Vec::new(),
        raw: json!({"key": "DEMO-2"}),
    }
}

#[test]
fn renders_jira_empty_ready_filtered_and_unmarked_browse_states() {
    let temp = tempfile::TempDir::new().expect("temp");
    let empty_ready = JiraDialog::new_ready(
        "project = DEMO".into(),
        50,
        temp.path().to_path_buf(),
        "https://jira.example.test".into(),
    );
    let mut app = jira_app(temp.path(), empty_ready.clone());
    assert!(render_text(&mut app).contains("No Jira issues loaded"));

    let mut filtered = empty_ready;
    filtered.filter = "DEMO".into();
    filtered.issues = vec![sample_unmarked_issue()];
    app.set_overlay(Overlay::Jira(Box::new(filtered)));
    let screen = render_text(&mut app);
    assert!(screen.contains("[ ] DEMO-2") && screen.contains("Filter: DEMO"));
}
